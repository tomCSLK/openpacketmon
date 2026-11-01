//! The notification sounds: the same three short sweeps the browser pages made with Web Audio.

use std::f32::consts::TAU;
use std::num::NonZero;
use std::time::Duration;

use rodio::buffer::SamplesBuffer;
use rodio::{DeviceSinkBuilder, Source};

use crate::notifications::types::sound::Sound;

const SAMPLE_RATE: u32 = 44_100;

enum Wave {
    Sine,
    Triangle,
    Sawtooth,
}

struct Shape {
    from: f32,
    to: f32,
    seconds: f32,
    wave: Wave,
}

fn shape(sound: Sound) -> Option<Shape> {
    match sound {
        Sound::Gulp => Some(Shape {
            from: 520.0,
            to: 180.0,
            seconds: 0.18,
            wave: Wave::Sine,
        }),
        Sound::Pop => Some(Shape {
            from: 900.0,
            to: 700.0,
            seconds: 0.07,
            wave: Wave::Triangle,
        }),
        Sound::Swhoosh => Some(Shape {
            from: 250.0,
            to: 1400.0,
            seconds: 0.35,
            wave: Wave::Sawtooth,
        }),
        Sound::None => None,
    }
}

/// The samples of a sound at `volume` (0–100)
fn samples(shape: &Shape, volume: u8) -> Vec<f32> {
    let n = (shape.seconds * SAMPLE_RATE as f32) as usize;
    let quieter = if matches!(shape.wave, Wave::Sawtooth) {
        0.35
    } else {
        1.0
    };
    let peak = 0.4 * f32::from(volume) / 100.0 * quieter;
    let ratio = shape.to / shape.from;
    let mut phase = 0.0_f32;
    (0..n)
        .map(|i| {
            let at = i as f32 / n as f32;
            let frequency = shape.from * ratio.powf(at);
            phase = (phase + frequency / SAMPLE_RATE as f32).fract();
            let value = match shape.wave {
                Wave::Sine => (phase * TAU).sin(),
                Wave::Triangle => 4.0 * (phase - 0.5).abs() - 1.0,
                Wave::Sawtooth => 2.0 * phase - 1.0,
            };
            // fades in within 10 ms and out to the end, without a click
            let seconds = i as f32 / SAMPLE_RATE as f32;
            let envelope = (seconds / 0.01).min(1.0) * (1.0 - at);
            value * peak * envelope
        })
        .collect()
}

/// Plays a sound in the background; without an audio device nothing happens
pub fn play(sound: Sound, volume: u8) {
    let Some(shape) = shape(sound).filter(|_| volume > 0) else {
        return;
    };
    let _ = std::thread::Builder::new()
        .name("thread_window_sound".to_owned())
        .spawn(move || {
            let (Ok(mut sink), Some(channels), Some(rate)) = (
                DeviceSinkBuilder::open_default_sink(),
                NonZero::new(1_u16),
                NonZero::new(SAMPLE_RATE),
            ) else {
                return;
            };
            sink.log_on_drop(false);
            let source = SamplesBuffer::new(channels, rate, samples(&shape, volume));
            let length = source
                .total_duration()
                .unwrap_or(Duration::from_secs_f32(shape.seconds));
            sink.mixer().add(source);
            std::thread::sleep(length + Duration::from_millis(60));
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sounds_stay_quiet_and_end_softly() {
        for sound in [Sound::Gulp, Sound::Pop, Sound::Swhoosh] {
            let s = samples(&shape(sound).unwrap(), 100);
            assert!(!s.is_empty());
            let peak = s.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
            assert!(peak > 0.05 && peak <= 0.4, "{sound}: {peak}");
            assert!(s.last().unwrap().abs() < 0.01, "{sound}");
        }
        assert!(shape(Sound::None).is_none());
    }
}
