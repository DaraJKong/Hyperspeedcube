//! Rendering logic.

use cgmath::{Matrix4, Rad, Vector4};
use glium::{BackfaceCullingMode, DrawParameters, Surface};
use std::collections::HashSet;

mod cache;
mod shaders;
mod verts;

use crate::colors;
use crate::config::get_config;
use crate::puzzle::{traits::*, PuzzleController, PuzzleEnum};
use crate::DISPLAY;
use cache::RenderCache;
use verts::*;

// const OUTLINE_COLOR: Option<[f32; 4]> = None;
const OUTLINE_COLOR: Option<[f32; 4]> = Some(colors::OUTLINE_BLACK);
// const OUTLINE_COLOR: Option<[f32; 4]> = colors::OUTLINE_WHITE;
const LINE_WIDTH: f32 = 2.0;

pub fn draw_puzzle(target: &mut glium::Frame, puzzle: &PuzzleEnum) -> Result<(), glium::DrawError> {
    match puzzle {
        PuzzleEnum::Rubiks3D(cube) => _draw_puzzle(target, cube),
        PuzzleEnum::Rubiks4D(cube) => _draw_puzzle(target, cube),
    }
}

fn setup_puzzle<P: PuzzleTrait>(cache: &mut RenderCache) {
    cache.last_puzzle_type = Some(P::TYPE);

    let mut surface_indices = vec![];
    let mut outline_indices = vec![];
    let mut base = 0;
    for _ in P::Sticker::iter() {
        // Prepare triangle indices.
        surface_indices.extend(P::Sticker::SURFACE_INDICES.iter().map(|&i| base + i));
        // Prepare line indices.
        outline_indices.extend(P::Sticker::OUTLINE_INDICES.iter().map(|&i| base + i));
        base += P::Sticker::VERTEX_COUNT;
    }
    // Write triangle indices.
    cache
        .tri_indices
        .slice(surface_indices.len())
        .write(&surface_indices);
    // Write line indices.
    cache
        .line_indices
        .slice(outline_indices.len())
        .write(&outline_indices);
}

fn _draw_puzzle<P: PuzzleTrait>(
    target: &mut glium::Frame,
    puzzle: &PuzzleController<P>,
) -> Result<(), glium::DrawError> {
    let config = get_config();

    let mut cache_ = cache::borrow_cache();
    let cache = &mut *cache_;

    let sticker_scale = 1.0 - config.gfx.sticker_spacing;
    // let face_scale = config.gfx.face_scale + (1.0 - sticker_scale) / 6.0;
    let face_scale = (1.0 - config.gfx.face_spacing) * 3.0 / (2.0 + sticker_scale);

    let geometry_params = GeometryParams {
        sticker_scale,
        face_scale,
    };

    let (target_w, target_h) = target.get_dimensions();
    target.clear_color_srgb_and_depth(colors::get_bg(), 1.0);

    if cache.last_puzzle_type != Some(P::TYPE) {
        setup_puzzle::<P>(&mut *cache);
    }

    // Prepare model matrices, which must be done here on the CPU so that we can do proper Z ordering.
    let stationary_model_matrix =
        Matrix4::from_angle_x(Rad(config.gfx.theta)) * Matrix4::from_angle_y(Rad(config.gfx.phi));
    let moving_model_matrix;
    let moving_pieces: HashSet<P::Piece>;
    if let Some((twist, progress)) = puzzle.current_twist() {
        moving_model_matrix = stationary_model_matrix * twist.matrix(progress);
        moving_pieces = twist.pieces().collect();
    } else {
        moving_model_matrix = stationary_model_matrix;
        moving_pieces = HashSet::new();
    };

    // Each sticker has a Vec<StickerVertex> with all of its vertices and a
    // single f32 containing the average Z value.
    let mut verts_by_sticker: Vec<(Vec<StickerVertex>, f32)> = vec![];
    // Generate vertices.
    for piece in P::Piece::iter() {
        let matrix = if moving_pieces.contains(&piece) {
            moving_model_matrix
        } else {
            stationary_model_matrix
        };

        for sticker in piece.stickers() {
            let [r, g, b] = puzzle.displayed().get_sticker(sticker).color();
            let color = [r, g, b, config.gfx.opacity];
            let mut sticker_verts = vec![];

            let radius = P::radius(geometry_params);
            let pre_scale = 1.0 / radius;
            let post_scale = 1.0 / pre_scale / (radius * radius * P::NDIM as f32).sqrt();
            let mut z_sum = 0.0;
            let mut w_sum = 0.0;
            for vert_pos in sticker.verts(geometry_params) {
                let pos = matrix * Vector4::from(vert_pos) * pre_scale;
                let w = -pos.w;
                w_sum += w;
                let mut pos = pos.truncate() * post_scale;
                if P::NDIM == 4 {
                    let fov = config.gfx.fov_4d;
                    pos *= post_scale / (1.0 + (fov.signum() + w) * (fov / 2.0).tan());
                };
                z_sum += pos.z;
                let pos = pos.extend(1.0).into(); // w = 1.0

                sticker_verts.push(StickerVertex { pos, color });
            }

            let avg_z = z_sum / sticker_verts.len() as f32;
            let avg_w = w_sum / sticker_verts.len() as f32;

            // Clip W coordinates too close to the camera.
            if avg_z.is_finite() && avg_w > -0.99 {
                verts_by_sticker.push((sticker_verts, avg_z));
            }
        }
    }
    let sticker_count = verts_by_sticker.len();
    // Sort by average Z position for proper transparency.
    verts_by_sticker.sort_by(|(_, z1), (_, z2)| z1.partial_cmp(z2).unwrap());
    let verts: Vec<StickerVertex> = verts_by_sticker
        .into_iter()
        .flat_map(|(verts, _)| verts)
        .collect();

    // Write sticker vertices.
    cache.stickers_vbo.slice(verts.len()).write(&verts);

    // To avoid dealing with 5x5 matrices, we'll do translation and rotation in
    // GLSL in separate steps.

    // Create the perspective matrix.
    let perspective_matrix: [[f32; 4]; 4] = {
        let min_dimen = std::cmp::min(target_w, target_h) as f32;
        let scale = min_dimen * config.gfx.scale;

        let xx = scale / target_w as f32;
        let yy = scale / target_h as f32;

        let fov = config.gfx.fov_3d;
        let zw = (fov / 2.0).tan(); // `tan(fov/2)` is the factor of how much the Z coordinate affects the XY coordinates.
        let ww = 1.0 + fov.signum() * zw;

        [
            [xx, 0.0, 0.0, 0.0],
            [0.0, yy, 0.0, 0.0],
            [0.0, 0.0, -1.0, -zw],
            [0.0, 0.0, 0.0, ww],
        ]
    };

    let draw_params = DrawParameters {
        blend: glium::Blend::alpha_blending(),
        smooth: Some(glium::Smooth::Nicest),
        depth: glium::Depth {
            test: glium::DepthTest::IfLessOrEqual,
            write: true,
            ..glium::Depth::default()
        },
        line_width: Some(LINE_WIDTH),
        backface_culling: BackfaceCullingMode::CullClockwise,
        ..DrawParameters::default()
    };

    let override_color: [f32; 4] = OUTLINE_COLOR.unwrap_or_default();

    // Draw triangles.
    let sticker_verts = cache.stickers_vbo.slice(verts.len());
    let tri_indices = cache
        .tri_indices
        .slice(P::Sticker::SURFACE_INDICES.len() * sticker_count);
    target.draw(
        sticker_verts,
        tri_indices,
        &shaders::BASIC,
        &uniform! {
            use_override_color: false,
            override_color: override_color,
            perspective_matrix: perspective_matrix,
        },
        &draw_params,
    )?;

    // Draw smooth outline.
    target.draw(
        cache.stickers_vbo.slice(verts.len()),
        cache
            .line_indices
            .slice(P::Sticker::OUTLINE_INDICES.len() * sticker_count),
        &shaders::BASIC,
        &uniform! {
            use_override_color: OUTLINE_COLOR.is_some(),
            override_color: override_color,
            perspective_matrix: perspective_matrix,
        },
        &draw_params,
    )?;

    Ok(())
}
