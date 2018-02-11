use webgl::*;
use uni_app::App;

use na::*;
use std::rc::{Rc, Weak};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::core::{Component, ComponentBased};
use super::{Camera, DirectionalLight, GameObject, Light, Material, Mesh, ShaderProgram, Texture};
use super::asset::{AssetDatabase, AssetSystem};

use super::imgui;

pub trait IEngine {
    fn new_gameobject(&mut self) -> Rc<RefCell<GameObject>>;

    fn asset_system<'a>(&'a self) -> &'a AssetSystem;

    fn gui_context(&mut self) -> Rc<RefCell<imgui::Context>>;
}

pub struct Engine<A = AssetDatabase>
where
    A: AssetSystem,
{
    pub gl: WebGLRenderingContext,
    pub main_camera: Option<Camera>,

    pub objects: Vec<Weak<RefCell<GameObject>>>,
    pub program_cache: RefCell<HashMap<&'static str, Rc<ShaderProgram>>>,
    pub asset_system: Rc<A>,

    pub gui_context: Rc<RefCell<imgui::Context>>,
}

#[derive(Default)]
struct EngineContext {
    mesh: Option<u64>,
    prog: Option<Rc<ShaderProgram>>,
    tex: Option<Rc<Texture>>,

    light: Option<Arc<Component>>,

    switch_mesh: u32,
    switch_prog: u32,
    switch_tex: u32,
}

impl EngineContext {
    pub fn need_prepare_program(&self, prog: &Rc<ShaderProgram>) -> bool {
        return self.prog.is_none() || (!Rc::ptr_eq(prog, self.prog.as_ref().unwrap()));
    }

    pub fn need_prepare_texture(&self, tex: &Rc<Texture>) -> bool {
        return self.tex.is_none() || (!Rc::ptr_eq(tex, self.tex.as_ref().unwrap()));
    }
}

impl<A> Engine<A>
where
    A: AssetSystem,
{
    pub fn clear(&self) {
        self.gl.clear(BufferBit::Color);
        self.gl.clear(BufferBit::Depth);
        self.gl.clear_color(0.2, 0.2, 0.2, 1.0);
    }

    fn setup_material(&self, ctx: &mut EngineContext, material: &Material) -> bool {
        let need_prepare = ctx.need_prepare_program(&material.program);
        if need_prepare {
            // Use the combined shader program object
            let p = material.program.clone();
            p.bind(&self.gl);
            ctx.prog = Some(p);
            ctx.switch_prog += 1;
        }

        let need_prepare = ctx.need_prepare_texture(&material.texture);
        if need_prepare {
            let curr = &mut ctx.prog;
            // Binding texture
            if !material.texture.bind(&self.gl, curr.as_ref().unwrap()) {
                return false;
            }
            ctx.tex = Some(material.texture.clone());
            ctx.switch_tex += 1;
        }

        // temp set the material shiness here
        if let Some(ref prog) = ctx.prog {
            prog.set("uShininess", 32.0);
        }

        true
    }

    fn render_object(
        &self,
        gl: &WebGLRenderingContext,
        ctx: &mut EngineContext,
        object: &GameObject,
        camera: &Camera,
    ) {
        // Setup Matrices
        let mut modelm = object.transform.to_homogeneous();
        modelm = modelm * Matrix4::new_nonuniform_scaling(&object.scale);

        let prog = ctx.prog.as_ref().unwrap();

        prog.set("uMVMatrix", camera.v * modelm);
        prog.set("uPMatrix", camera.p);
        prog.set("uNMatrix", modelm.try_inverse().unwrap().transpose());
        prog.set("uMMatrix", modelm);
        prog.set("uViewPos", camera.eye());

        {
            let light_com = ctx.light.as_ref().unwrap();
            let light = light_com.try_into::<Light>().unwrap();
            let light_br = light.borrow();

            // We must have at least one direction light.
            let dir_light = light_br.directional().unwrap();

            prog.set("uDirectionalLight.direction", dir_light.direction);
            prog.set("uDirectionalLight.ambient", dir_light.ambient);
            prog.set("uDirectionalLight.diffuse", dir_light.diffuse);
            prog.set("uDirectionalLight.specular", dir_light.specular);
        }

        // Setup Mesh
        let (mesh_ref, com) = object.find_component::<Mesh>().unwrap();

        let mesh = mesh_ref.borrow();

        if ctx.mesh.is_none() || ctx.mesh.unwrap() != com.id() {
            mesh.bind(&self.gl, prog);
            ctx.switch_mesh += 1;
        }

        prog.commit(gl);
        mesh.render(gl);
    }

    pub fn begin(&mut self) {
        imgui::begin();
    }

    pub fn end(&mut self) {}

    fn find_component<T>(&self) -> Option<Arc<Component>>
    where
        T: 'static + ComponentBased,
    {
        let objects = &self.objects;
        for obj in objects.iter() {
            let r = obj.upgrade().map_or(None, |obj| {
                let object = obj.borrow();
                match object.find_component::<T>() {
                    Some((_, c)) => Some(c),
                    None => None,
                }
            });

            if r.is_some() {
                return r;
            }
        }

        None
    }

    pub fn render(&mut self) {
        self.clear();
        imgui::pre_render(self);

        let gl = &self.gl;
        let objects = &self.objects;
        if let &Some(camera) = &self.main_camera.as_ref() {
            let mut ctx: EngineContext = Default::default();

            // prepare light.
            ctx.light = Some(self.find_component::<Light>().unwrap_or({
                Component::new(Light::Directional(DirectionalLight {
                    direction: Vector3::new(0.5, -1.0, 1.0).normalize(),
                    ambient: Vector3::new(0.2, 0.2, 0.2),
                    diffuse: Vector3::new(0.5, 0.5, 0.5),
                    specular: Vector3::new(1.0, 1.0, 1.0),
                }))
            }));

            for obj in objects.iter() {
                obj.upgrade().map(|obj| {
                    let object = obj.borrow();
                    if let Some((material_ref, _)) = object.find_component::<Material>() {
                        let material = material_ref.borrow();

                        if self.setup_material(&mut ctx, &material) {
                            self.render_object(gl, &mut ctx, &object, camera);

                            let (_, meshcom) = object.find_component::<Mesh>().unwrap();
                            ctx.mesh = Some(meshcom.id());
                        }
                    }
                });
            }
        }

        // drop all gameobjects if there are only references
        self.objects.retain(|obj| obj.upgrade().is_some());
    }

    pub fn new(app: &App, size: (u32, u32)) -> Engine<A> {
        let gl = WebGLRenderingContext::new(app.canvas());

        /*=========Drawing the triangle===========*/

        // Clear the canvas
        gl.clear_color(0.5, 0.5, 0.5, 1.0);

        // Enable the depth test
        gl.enable(Flag::DepthTest);

        // Enable alpha blending
        gl.enable(Flag::Blend);

        // Clear the color buffer bit
        gl.clear(BufferBit::Color);
        gl.clear(BufferBit::Depth);
        gl.blend_func(BlendMode::SrcAlpha, BlendMode::OneMinusSrcAlpha);

        // Set the view port
        gl.viewport(0, 0, size.0, size.1);

        Engine {
            gl: gl,
            main_camera: None,
            objects: vec![],
            program_cache: RefCell::new(HashMap::new()),
            asset_system: Rc::new(A::new()),
            gui_context: Rc::new(RefCell::new(imgui::Context::new(size.0, size.1))),
        }
    }
}

impl<A: AssetSystem> IEngine for Engine<A> {
    fn new_gameobject(&mut self) -> Rc<RefCell<GameObject>> {
        let go = Rc::new(RefCell::new(GameObject {
            transform: Isometry3::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            components: vec![],
        }));

        self.objects.push(Rc::downgrade(&go));
        go
    }

    fn gui_context(&mut self) -> Rc<RefCell<imgui::Context>> {
        self.gui_context.clone()
    }

    fn asset_system<'a>(&'a self) -> &'a AssetSystem {
        &*self.asset_system
    }
}