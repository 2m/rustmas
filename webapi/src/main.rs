mod db;
mod frame_broadcaster;

use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_web_actors::ws;
use db::Db;
use dotenvy::dotenv;
use log::{error, info};
use serde::Deserialize;
use serde_json::json;
use std::{env, error::Error};
use tokio::sync::{mpsc, Mutex};

use actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer};

use crate::frame_broadcaster::{FrameBroadcaster, FrameBroadcasterSession};

#[post("/events/restart")]
async fn restart_events(app_state: web::Data<AppState>) -> HttpResponse {
    app_state
        .animation_controller
        .lock()
        .await
        .restart_event_generators()
        .await;
    HttpResponse::Ok().json(())
}

#[get("/events/schema")]
async fn events_schema(app_state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(
        app_state
            .animation_controller
            .lock()
            .await
            .event_generator_parameter_schema()
            .await,
    )
}

#[post("/events/values")]
async fn set_event_parameters(
    params: web::Json<serde_json::Value>,
    app_state: web::Data<AppState>,
) -> HttpResponse {
    match app_state
        .animation_controller
        .lock()
        .await
        .set_event_generator_parameters(params.0)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(json!(())),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

#[derive(Deserialize)]
struct SwitchForm {
    animation: String,
    params: Option<serde_json::Value>,
}

async fn switch_inner(
    animation_id: &str,
    params: &Option<serde_json::Value>,
    app_state: web::Data<AppState>,
) -> HttpResponse {
    let mut controller = app_state.animation_controller.lock().await;
    if let Err(e) = controller.switch_animation(animation_id).await {
        return HttpResponse::InternalServerError().json(json!({ "error": format!("{:#}", e) }));
    }

    if let Some(params) = params {
        let _ = controller.set_parameters(params.clone()).await;
    } else if let Ok(Some(params)) = app_state.db.get_parameters(animation_id).await {
        let _ = controller.set_parameters(params).await;
    } else if let Ok(params) = controller.parameter_values().await {
        let _ = app_state.db.set_parameters(animation_id, &params).await;
    }

    *app_state.animation_id.lock().await = animation_id.to_owned();
    match controller.parameters().await {
        Ok(animation) => HttpResponse::Ok().json(json!({ "animation": animation })),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string() })),
    }
}

#[post("/reload")]
async fn reload(app_state: web::Data<AppState>) -> HttpResponse {
    let id = app_state.animation_id.lock().await.clone();
    switch_inner(&id, &None, app_state).await
}

#[post("/switch")]
async fn switch(form: web::Json<SwitchForm>, app_state: web::Data<AppState>) -> HttpResponse {
    switch_inner(&form.animation, &form.params, app_state).await
}

#[post("/turn_off")]
async fn turn_off(app_state: web::Data<AppState>) -> HttpResponse {
    app_state.animation_controller.lock().await.turn_off().await;
    HttpResponse::Ok().json(())
}

#[get("/params")]
async fn get_params(app_state: web::Data<AppState>) -> HttpResponse {
    match app_state
        .animation_controller
        .lock()
        .await
        .parameters()
        .await
    {
        Ok(animation) => HttpResponse::Ok().json(json!({ "animation": animation })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "error": e.to_string() })),
    }
}

#[post("/params/save")]
async fn save_params(app_state: web::Data<AppState>) -> HttpResponse {
    let parameter_values = match app_state
        .animation_controller
        .lock()
        .await
        .parameter_values()
        .await
    {
        Ok(params) => params,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({"error": e.to_string() }))
        }
    };

    match app_state
        .db
        .set_parameters(&app_state.animation_id.lock().await, &parameter_values)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(json!(())),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

#[post("/params/reset")]
async fn reset_params(app_state: web::Data<AppState>) -> HttpResponse {
    match app_state
        .db
        .get_parameters(&app_state.animation_id.lock().await)
        .await
    {
        Ok(Some(params)) => {
            let mut controller = app_state.animation_controller.lock().await;
            let _ = controller.set_parameters(params).await;
            match controller.parameters().await {
                Ok(animation) => HttpResponse::Ok().json(json!({ "animation": animation })),
                Err(e) => {
                    HttpResponse::InternalServerError().json(json!({"error": e.to_string() }))
                }
            }
        }
        Ok(None) => HttpResponse::InternalServerError()
            .json(json!({"error": "No parameters stored for this animation"})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

#[post("/params")]
async fn post_params(
    params: web::Json<serde_json::Value>,
    app_state: web::Data<AppState>,
) -> HttpResponse {
    match app_state
        .animation_controller
        .lock()
        .await
        .set_parameters(params.0)
        .await
    {
        Ok(_) => HttpResponse::Ok().json(json!(())),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

#[post("/discover")]
async fn discover(app_state: web::Data<AppState>) -> HttpResponse {
    let mut controller = app_state.animation_controller.lock().await;
    match controller.discover_animations() {
        Ok(_) => HttpResponse::Ok().json(json!({
            "animations": controller
                .list_animations()
                .iter()
                .map(|(id, plugin)| json!({"id": id, "name": plugin.manifest.display_name}))
                .collect::<Vec<_>>()})),
        Err(e) => HttpResponse::InternalServerError().json(json!({"error": e.to_string()})),
    }
}

#[get("/list")]
async fn list(app_state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "animations": app_state
            .animation_controller
            .lock()
            .await
            .list_animations()
            .iter()
            .map(|(id, plugin)| json!({"id": id, "name": plugin.manifest.display_name}))
            .collect::<Vec<_>>()}))
}

#[get("/frames")]
async fn frames(
    req: HttpRequest,
    stream: web::Payload,
    server: web::Data<Addr<FrameBroadcaster>>,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(
        FrameBroadcasterSession::new(server.get_ref().clone()),
        &req,
        stream,
    )
}

#[get("/points")]
async fn points(app_state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "points": app_state.animation_controller.lock().await.points()
    }))
}

struct AppState {
    animation_controller: Mutex<rustmas_animator::Controller>,
    animation_id: Mutex<String>,
    db: Db,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    env_logger::init();

    let (sender, receiver) = mpsc::channel::<lightfx::Frame>(1);

    info!("Starting controller");
    let controller = {
        let mut builder = rustmas_animator::Controller::builder()
            .points_from_file(&env::var("RUSTMAS_POINTS_PATH").unwrap_or("lights.csv".to_owned()))?
            .lights_feedback(sender)
            .plugin_dir(env::var("RUSTMAS_PLUGIN_DIR").unwrap_or(".".to_owned()));

        if let Ok(url) = env::var("RUSTMAS_LIGHTS_URL") {
            if url.starts_with("http://") {
                builder = builder.http_lights(&url)?;
            } else if url.starts_with("tcp://") {
                builder = builder.tcp_lights(&url)?;
            } else if url.starts_with("udp://") {
                builder = builder.udp_lights(&url)?;
            } else {
                error!("Unknown remote client protocol, ignoring");
            }
        }
        if env::var("RUSTMAS_USE_TTY")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false)
        {
            builder = builder.local_lights()?;
        }

        let mut controller = builder.build();
        controller.discover_animations()?;
        info!("Discovered {} plugins", controller.list_animations().len());
        controller
    };

    info!("Establishing database connection");
    let db = Db::new(&env::var("RUSTMAS_DB_PATH").unwrap_or("db.sqlite".to_owned())).await?;

    info!("Starting http server");
    let app_state = web::Data::new(AppState {
        animation_controller: Mutex::new(controller),
        animation_id: Mutex::new("blank".to_owned()),
        db,
    });

    let frame_broadcaster = web::Data::new(FrameBroadcaster::new(receiver).start());

    HttpServer::new(move || {
        let cors = Cors::permissive();
        App::new()
            .wrap(cors)
            .service(restart_events)
            .service(events_schema)
            .service(set_event_parameters)
            .service(reload)
            .service(switch)
            .service(turn_off)
            .service(list)
            .service(discover)
            .service(get_params)
            .service(post_params)
            .service(save_params)
            .service(reset_params)
            .service(frames)
            .service(points)
            .app_data(app_state.clone())
            .app_data(frame_broadcaster.clone())
    })
    .bind(("0.0.0.0", 8081))?
    .run()
    .await?;

    Ok(())
}
