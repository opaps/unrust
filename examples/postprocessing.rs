#![recursion_limit = "512"]
#![feature(integer_atomics)]

/* common */
extern crate futures;
extern crate nalgebra as na;
extern crate uni_app;
extern crate unigame;

mod appfs;

use appfs::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::ops::{Deref, DerefMut};
use na::{Point3, UnitQuaternion, Vector3};
use std::collections::HashMap;
use std::sync::{Arc, Weak};

use unigame::engine::*;
use uni_app::{App, AppConfig, AppEvent, FPS};

type Handle<T> = Rc<RefCell<T>>;

struct Game {
    list: Vec<Handle<GameObject>>,
    engine: AppEngine,
    point_light_coms: Vec<Weak<Component>>,
    rt: Rc<RenderTexture>,
}

impl Game {
    fn new(engine: AppEngine) -> Game {
        let mut g = Game {
            list: Vec::new(),
            engine: engine,
            point_light_coms: Vec::new(),
            rt: Rc::new(RenderTexture::new(1024, 1024)),
        };

        g.setup();
        g
    }

    pub fn step(&mut self) {
        self[5]
            .borrow_mut()
            .transform
            .append_rotation_mut(&UnitQuaternion::new(Vector3::new(0.01, 0.02, 0.005)));
    }

    pub fn reset(&mut self) {
        self.list.clear();
        self.engine.asset_system_mut().reset();
        self.point_light_coms.clear();

        self.setup();
    }

    pub fn setup(&mut self) {
        // add direction light to scene.
        let _dir_light_com = {
            let go = self.engine.new_gameobject();
            // Make sure it is store some where, else it will gc
            self.push(go.clone());

            let mut go_mut = go.borrow_mut();
            let com = go_mut.add_component(Light::new(Directional {
                direction: Vector3::new(0.5, -1.0, 1.0).normalize(),
                ambient: Vector3::new(0.2, 0.2, 0.2),
                diffuse: Vector3::new(0.5, 0.5, 0.5),
                specular: Vector3::new(1.0, 1.0, 1.0),
            }));

            com
        };

        // Add 4 points light to scene
        let point_light_positions = vec![
            Vector3::new(-30.0, 30.0, -30.0),
            Vector3::new(-15.0, 300.0, -10.0),
            Vector3::new(30.0, 50.0, 30.0),
            Vector3::new(30.0, 100.0, -20.0),
        ];

        for p in point_light_positions.into_iter() {
            let go = self.engine.new_gameobject();
            // Make sure it is store some where, else it will gc
            self.push(go.clone());

            let mut go_mut = go.borrow_mut();
            let com = Light::new(Point {
                position: p,
                ambient: Vector3::new(0.05, 0.05, 0.05),
                diffuse: Vector3::new(0.8, 0.8, 0.8),
                specular: Vector3::new(1.0, 1.0, 1.0),
                constant: 1.0,
                linear: 0.022,
                quadratic: 0.0019,
            });

            self.point_light_coms
                .push(Arc::downgrade(&go_mut.add_component(com)));
        }

        let go = { self.engine.new_gameobject() };
        {
            let db = &mut self.engine.asset_system();
            let mut go_mut = go.borrow_mut();
            let mut params = HashMap::new();
            params.insert(
                "uMaterial.diffuse".to_string(),
                MaterialParam::Texture(db.new_texture("tex_a.png")),
            );
            params.insert(
                "uMaterial.shininess".to_string(),
                MaterialParam::Float(32.0),
            );
            let material = Material::new(db.new_program("phong"), params);

            let mut mesh = Mesh::new();
            mesh.add_surface(db.new_mesh_buffer("cube"), Rc::new(material));
            go_mut.add_component(mesh);
        }
        self.list.push(go.clone());

        let screen_quad = { self.engine.new_gameobject() };
        {
            let db = &mut self.engine.asset_system();
            let mut go_mut = screen_quad.borrow_mut();
            let mut params = HashMap::new();
            params.insert(
                "uDiffuse".to_string(),
                MaterialParam::Texture(self.rt.as_texture()),
            );

            let material = Material::new(db.new_program("crt"), params);
            let mut mesh = Mesh::new();
            mesh.add_surface(db.new_mesh_buffer("screen_quad"), Rc::new(material));
            go_mut.add_component(mesh);
        }
        self.list.push(screen_quad.clone());
    }
}

impl Deref for Game {
    type Target = Vec<Handle<GameObject>>;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for Game {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}

pub fn main() {
    let size = (800, 600);
    let config = AppConfig::new("Postprocessing demo", size);
    let app = App::new(config);
    {
        let mut game = Game::new(Engine::new(app.canvas(), size));
        game.engine.main_camera = Some(Rc::new(RefCell::new(Camera::new())));

        use imgui::Metric::*;

        let mut fps = FPS::new();
        let mut last_event = None;
        let mut eye = Vector3::new(-3.0, 3.0, -3.0);
        let up = Vector3::new(0.0, 1.0, 0.0);

        app.run(move |app: &mut App| {
            game.engine.begin();
            fps.step();
            game.step();

            // Handle Events
            {
                let target = Vector3::new(0.0, 0.0, 0.0);
                let front = (eye - target).normalize();

                let events = app.events.borrow();
                for evt in events.iter() {
                    last_event = Some(evt.clone());
                    match evt {
                        &AppEvent::Click(_) => {}

                        &AppEvent::KeyDown(ref key) => {
                            match key.code.as_str() {
                                "KeyA" => eye = na::Rotation3::new(up * -0.02) * eye,
                                "KeyD" => eye = na::Rotation3::new(up * 0.02) * eye,
                                "KeyW" => eye = eye - front * 2.0,
                                "KeyS" => eye = eye + front * 2.0,
                                "Escape" => game.reset(),
                                _ => (),
                            };
                        }

                        _ => (),
                    }
                }
            }

            // Update Camera
            {
                let mut cam = game.engine.main_camera.as_ref().unwrap().borrow_mut();
                cam.lookat(
                    &Point3::from_coordinates(eye),
                    &Point3::new(0.0, 0.0, 0.0),
                    &Vector3::new(0.0, 1.0, 0.0),
                );
            }

            // Update Light
            for light_com_weak in game.point_light_coms.iter() {
                if let Some(light_com) = light_com_weak.upgrade() {
                    if let Some(lr) = light_com.try_as::<Light>() {
                        let mut light = lr.borrow_mut();
                        let mut pos = light.point().unwrap().position;

                        light.point_mut().unwrap().position = na::Rotation3::new(up * 0.02) * pos;
                    }
                }
            }

            // Setup fb for camera
            {
                let mut cam = game.engine.main_camera.as_ref().unwrap().borrow_mut();
                cam.render_texture = Some(game.rt.clone());

                // Setup proper viewport to render to the whole texture
                cam.rect = Some(((0, 0), (1024, 1024)));
                // show only cube
                game.list[5].borrow_mut().active = true;
                game.list[6].borrow_mut().active = false;
                imgui::pivot((0.0, 0.0));
                imgui::label(
                    Native(0.0, 0.0) + Pixel(8.0, 8.0),
                    &format!("fps: {} nobj: {}", fps.fps, game.engine.objects.len()),
                );

                imgui::pivot((1.0, 1.0));
                imgui::label(
                    Native(1.0, 1.0) - Pixel(8.0, 8.0),
                    "[Esc]  : reload all (include assets)",
                );

                imgui::pivot((1.0, 0.0));
                imgui::label(
                    Native(1.0, 0.0) + Pixel(-8.0, 8.0),
                    &format!("last event: {:?}", last_event),
                );
                // Render current scene by camera using given frame buffer
                game.engine.render_pass(&cam);

                // Clean up stuffs in camera, as later we could render normally
                cam.render_texture = None;
                cam.rect = None;
            }
            // show only screen_quad
            game.list[5].borrow_mut().active = false;
            game.list[6].borrow_mut().active = true;
            // Render
            game.engine.render();

            // End
            game.engine.end();
        });
    }
}