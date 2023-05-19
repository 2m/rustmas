use std::f64::consts::PI;

use animation_api::Animation;
use animation_utils::decorators::{BrightnessControlled, SpeedControlled};
use animation_utils::ParameterSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Serialize, Deserialize, ParameterSchema)]
pub struct Parameters {
    #[schema_field(name = "First color", color)]
    color_a: lightfx::Color,

    #[schema_field(name = "Second color", color)]
    color_b: lightfx::Color,

    #[schema_field(name = "Twistiness", number(min = "-5.0", max = 5.0, step = 0.02))]
    twistiness: f64,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            color_a: lightfx::Color::rgb(255, 0, 0),
            color_b: lightfx::Color::rgb(255, 255, 255),
            twistiness: 1.0,
        }
    }
}

#[animation_utils::plugin]
pub struct BarberPole {
    points_polar: Vec<(f64, f64, f64)>,
    time: f64,
    parameters: Parameters,
}

impl BarberPole {
    pub fn create(points: Vec<(f64, f64, f64)>) -> impl Animation {
        SpeedControlled::new(BrightnessControlled::new(Self {
            points_polar: points.into_iter().map(animation_utils::to_polar).collect(),
            time: 0.0,
            parameters: Default::default(),
        }))
    }
}

impl Animation for BarberPole {
    type Parameters = Parameters;

    fn update(&mut self, delta: f64) {
        self.time += delta;
    }

    fn render(&self) -> lightfx::Frame {
        self.points_polar
            .iter()
            .map(|(_, a, h)| {
                if animation_utils::cycle(a / PI + self.time + h * self.parameters.twistiness, 2.0)
                    < 1.0
                {
                    self.parameters.color_a
                } else {
                    self.parameters.color_b
                }
            })
            .into()
    }

    fn animation_name(&self) -> &str {
        "Barber Pole"
    }

    fn set_parameters(&mut self, parameters: Self::Parameters) {
        self.parameters = parameters;
    }

    fn get_parameters(&self) -> Self::Parameters {
        self.parameters.clone()
    }
}
