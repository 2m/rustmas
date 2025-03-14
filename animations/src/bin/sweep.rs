use animation_api::Animation;
use animation_utils::decorators::{BrightnessControlled, SpeedControlled};
use animation_utils::{EnumSchema, Schema};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Clone, Default, Serialize, Deserialize, EnumSchema)]
enum Direction {
    #[schema_variant(name = "Bottom to top")]
    #[default]
    BottomToTop,

    #[schema_variant(name = "Top to bottom")]
    TopToBottom,

    #[schema_variant(name = "Back to front")]
    BackToFront,

    #[schema_variant(name = "Front to back")]
    FrontToBack,

    #[schema_variant(name = "Left to right")]
    LeftToRight,

    #[schema_variant(name = "Right to left")]
    RightToLeft,
}

#[derive(Clone, Serialize, Deserialize, Schema)]
pub struct Parameters {
    #[schema_field(name = "Direction", enum_options)]
    direction: Direction,

    #[schema_field(
        name = "Band size",
        description = "Thickness of the sweep band",
        number(min = 0.0, max = 2.0, step = 0.05)
    )]
    band_size: f64,

    #[schema_field(name = "Band color", description = "Color of the sweep band", color)]
    color: lightfx::Color,
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            direction: Direction::BottomToTop,
            band_size: 0.2,
            color: lightfx::Color::white(),
        }
    }
}

#[animation_utils::plugin]
pub struct Sweep {
    points: Vec<(f64, f64, f64)>,
    time: f64,
    parameters: Parameters,
}

impl Sweep {}

impl Animation for Sweep {
    type Parameters = Parameters;
    type Wrapped = SpeedControlled<BrightnessControlled<Self>>;

    fn new(points: Vec<(f64, f64, f64)>) -> Self {
        Self {
            points,
            time: 0.0,
            parameters: Default::default(),
        }
    }

    fn update(&mut self, delta: f64) {
        self.time += delta;
    }

    fn render(&self) -> lightfx::Frame {
        let time =
            self.time % (2.0 + self.parameters.band_size) - (1.0 + self.parameters.band_size / 2.0);
        self.points
            .iter()
            .map(|(x, y, z)| match self.parameters.direction {
                Direction::BottomToTop => *y,
                Direction::TopToBottom => -*y,
                Direction::BackToFront => -*z,
                Direction::FrontToBack => *z,
                Direction::LeftToRight => *x,
                Direction::RightToLeft => -*x,
            })
            .map(|h| {
                if h > time && h < time + self.parameters.band_size {
                    self.parameters.color
                } else {
                    lightfx::Color::black()
                }
            })
            .into()
    }

    fn set_parameters(&mut self, parameters: Self::Parameters) {
        self.parameters = parameters;
    }

    fn get_parameters(&self) -> Self::Parameters {
        self.parameters.clone()
    }
}
