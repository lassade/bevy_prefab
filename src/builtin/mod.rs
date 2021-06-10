use bevy::{prelude::*, render::render_graph::base::MainPass};

pub mod objects;
pub mod primitives;

/// Equivalent to [`PbrBundle`] but without the transforms, mesh and material components
#[derive(Bundle)]
struct PbrPrimitiveBundle {
    main_pass: MainPass,
    draw: Draw,
    visible: Visible,
    render_pipelines: RenderPipelines,
}

impl Default for PbrPrimitiveBundle {
    fn default() -> Self {
        let PbrBundle {
            mesh: _,
            material: _,
            main_pass,
            draw,
            visible,
            render_pipelines,
            transform: _,
            global_transform: _,
        } = Default::default();

        Self {
            main_pass,
            draw,
            visible,
            render_pipelines,
        }
    }
}
