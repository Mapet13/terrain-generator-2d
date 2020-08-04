use opensimplex_noise_rs::OpenSimplexNoise;

use image::{ImageBuffer, Rgb};

use rand::Rng;

use glutin_window::GlutinWindow as Window;
use opengl_graphics::{OpenGL, Texture};
use piston::event_loop::{EventSettings, Events};
use piston::input::RenderEvent;
use piston::{Button, window::WindowSettings, Key, PressEvent};

const IMAGE_SIZE: [i32; 2] = [2048, 2048];
const WIN_SCALE: f32 = 0.25;
const WIN_SIZE: [u32; 2] = [
    (IMAGE_SIZE[0] as f32 * WIN_SCALE) as u32,
    (IMAGE_SIZE[0] as f32 * WIN_SCALE) as u32,
];

fn sum_octaves(
    num_iterations: i32,
    point: (i32, i32),
    persistence: f64,
    scale: f64,
    low: f64,
    high: f64,
    noise_fn: impl Fn(f64, f64) -> f64,
) -> f64 {
    let mut max_amp = 0.0;
    let mut amp = 1.0;
    let mut freq = scale;
    let mut noise = 0.0;

    for _ in 0..num_iterations {
        noise += noise_fn(point.0 as f64 * freq, point.1 as f64 * freq) * amp;
        max_amp += amp;
        amp *= persistence;
        freq *= 2.0;
    }

    (noise / max_amp) * (high - low) / 2.0 + (high + low) / 2.0
}

fn generate_gradient() -> Vec<f32> {
    let mut gradient: Vec<f32> = vec![1.0; (IMAGE_SIZE[0] * IMAGE_SIZE[1]) as usize];

    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            let mut color_value: f32;

            let a = if x > (IMAGE_SIZE[0] / 2) {
                IMAGE_SIZE[0] - x
            } else {
                x
            };

            let b = if y > IMAGE_SIZE[1] / 2 {
                IMAGE_SIZE[1] - y
            } else {
                y
            };

            let smaller = std::cmp::min(a, b) as f32;
            color_value = smaller / (IMAGE_SIZE[0] as f32 / 2.0);

            color_value = 1.0 - color_value;
            color_value = color_value * color_value;

            gradient[get_id_from_pos(x, y)] = match color_value - 0.1 {
                x if x > 1.0 => 1.0,
                x if x < 0.0 => 0.0,
                x => x,
            };
        }
    }

    gradient
}

fn generate_maps(gradient: &[f32]) -> (Vec<f32>, Vec<f32>) {
    let mut rng = rand::thread_rng();

    let mut height_map = generate_noise_map(rng.gen_range(0, std::i64::MAX), 0.004);
    let mut biome_map = generate_noise_map(rng.gen_range(0, std::i64::MAX), 0.007);

    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            height_map[get_id_from_pos(x, y)] =
                height_map[get_id_from_pos(x, y)] * 1.1 - gradient[get_id_from_pos(x, y)] * 0.8;
            biome_map[get_id_from_pos(x, y)] =
                biome_map[get_id_from_pos(x, y)] - (0.1 - gradient[get_id_from_pos(x, y)]) * 0.4;
            if height_map[get_id_from_pos(x, y)] < 0.0 {
                height_map[get_id_from_pos(x, y)] = 0.0;
            }
            if biome_map[get_id_from_pos(x, y)] < 0.0 {
                biome_map[get_id_from_pos(x, y)] = 0.0;
            }
        }
    }

    (height_map, biome_map)
}

fn get_id_from_pos(x: i32, y: i32) -> usize {
    (x + IMAGE_SIZE[0] * y) as usize
}

fn generate_noise_map(seed: i64, scale: f64) -> Vec<f32> {
    let noise_generator = OpenSimplexNoise::new(Some(seed));

    let mut map: Vec<f32> = vec![0.0; (IMAGE_SIZE[0] * IMAGE_SIZE[1]) as usize];
    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            let val = sum_octaves(16, (x, y), 0.5, scale, 0.0, 1.0, |x, y| {
                noise_generator.eval_2d(x, y)
            });

            map[get_id_from_pos(x, y)] = val as f32;
        }
    }
    map
}

enum Biomes {
    Grass,
    DeepWater,
    Water,
    Dirt,
    Sand,
    WetSand,
    DarkForest,
    HighDarkForest,
    LightForest,
    Mountain,
    HighMountain,
    Snow,
}

fn get_biome_color(biome: Biomes) -> [u8; 3] {
    match biome {
        Biomes::Grass => [120, 157, 80],
        Biomes::Water => [9, 82, 198],
        Biomes::DeepWater => [0, 62, 178],
        Biomes::Dirt => [114, 98, 49],
        Biomes::Sand => [194, 178, 128],
        Biomes::WetSand => [164, 148, 99],
        Biomes::DarkForest => [60, 97, 20],
        Biomes::HighDarkForest => [40, 77, 0],
        Biomes::LightForest => [85, 122, 45],
        Biomes::Mountain => [140, 142, 123],
        Biomes::HighMountain => [160, 162, 143],
        Biomes::Snow => [235, 235, 235],
    }
}

fn generate_image(height_map: &[f32], biome_map: &[f32]) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    let mut image =
        ImageBuffer::<Rgb<u8>, Vec<u8>>::new(IMAGE_SIZE[0] as u32, IMAGE_SIZE[1] as u32);

    for x in 0..IMAGE_SIZE[0] {
        for y in 0..IMAGE_SIZE[1] {
            let height = height_map[get_id_from_pos(x, y)];
            let moisture = biome_map[get_id_from_pos(x, y)];

            let biome = match (height, moisture) {
                (a, _) if a < 0.39 => Biomes::DeepWater,
                (a, _) if a < 0.42 => Biomes::Water,
                (a, b) if a < 0.46 && b < 0.57 => Biomes::Sand,
                (a, b) if a < 0.47 && b < 0.6 => Biomes::WetSand,
                (a, b) if a < 0.47 && b >= 0.6 => Biomes::Dirt,
                (a, b) if a > 0.54 && b < 0.43 && a < 0.62 => Biomes::Grass,
                (a, b) if a < 0.62 && b >= 0.58 => Biomes::HighDarkForest,
                (a, b) if a < 0.62 && b >= 0.49 => Biomes::DarkForest,
                (a, _) if a >= 0.79 => Biomes::Snow,
                (a, _) if a >= 0.74 => Biomes::HighMountain,
                (a, b) if a >= 0.68 && b >= 0.10 => Biomes::Mountain,
                _ => Biomes::LightForest,
            };

            let color = get_biome_color(biome);
            let pixel = image.get_pixel_mut(x as u32, y as u32);
            *pixel = image::Rgb(color);
        }
    }

    image
}

fn main() {
    println!("Generating gradient...");
    let gradient = generate_gradient();
    println!("DONE");

    'running: loop {
        println!("Generating maps...");
        let (height_map, biome_map) = generate_maps(&gradient);
        println!("DONE");

        println!("Generating image...");
        let image = generate_image(&height_map, &biome_map);
        image.save("output.png").unwrap();
        println!("DONE");

        let opengl = OpenGL::V3_2;
        let mut window: Window =
            WindowSettings::new("Terrain map generator - 2d", [WIN_SIZE[0], WIN_SIZE[1]])
                .exit_on_esc(false)
                .graphics_api(opengl)
                .resizable(false)
                .build()
                .unwrap();

        let mut gl = opengl_graphics::GlGraphics::new(opengl);

        let settings = piston_window::TextureSettings::new();

        settings.filter(piston_window::Filter::Nearest);

        let texture = Texture::from_path(std::path::Path::new("output.png"), &settings).unwrap();

        let mut events = Events::new(EventSettings::new());
        while let Some(e) = events.next(&mut window) {
            if let Some(Button::Keyboard(key)) = e.press_args() {
                if key == Key::Space {
                    continue 'running;
                }
                if key == Key::Escape {
                    break 'running;
                }
            };
            if let Some(r) = e.render_args() {
                use graphics::*;

                gl.draw(r.viewport(), |c, g| {
                    clear([1.0; 4], g);

                    let transform = c.transform.zoom(WIN_SCALE as f64);

                    image(&texture, transform, g);
                });
            }
        }
    }
}
