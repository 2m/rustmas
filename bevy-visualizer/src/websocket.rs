use bevy::prelude::*;
use ewebsock::{WsEvent, WsMessage};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Mutex};

use crate::Led;

pub(crate) struct WebsocketPlugin {
    endpoint: String,
}

impl WebsocketPlugin {
    pub(crate) fn new(url: &str) -> Self {
        Self {
            endpoint: url.to_string(),
        }
    }
}

impl Plugin for WebsocketPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = mpsc::channel();
        ewebsock::ws_receive(
            self.endpoint.clone(),
            Box::new(move |event| {
                let _ = sender.send(event);
                std::ops::ControlFlow::Continue(())
            }),
        )
        .unwrap();
        app.insert_resource(Receiver(Mutex::new(receiver)))
            .add_system(listen_for_frame);
    }
}

struct Receiver(Mutex<mpsc::Receiver<WsEvent>>);
impl Resource for Receiver {}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Color {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FrameEvent {
    pixels: Vec<Color>,
}

fn listen_for_frame(
    recv: Res<Receiver>,
    query: Query<(&Handle<StandardMaterial>, &Led)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut last_frame = None;

    while let Ok(event) = recv.0.lock().unwrap().try_recv() {
        if let WsEvent::Message(WsMessage::Binary(bytes)) = event {
            last_frame = Some(bytes);
        }
    }

    if let Some(frame) = last_frame {
        let colors: Vec<_> = frame.into_iter().tuples::<(u8, u8, u8)>().collect();
        for (material, led) in query.iter() {
            let Some((r, g, b)) = colors.get(led.0) else {
                continue;
            };
            materials.get_mut(material).unwrap().base_color =
                bevy::prelude::Color::rgb_u8(*r, *g, *b);
        }
    }
}
