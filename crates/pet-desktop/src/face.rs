//! Surface-fitted reaction meshes, independent of private model source files.
use crate::avatar::Avatar;
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    mesh::{Indices, VertexAttributeValues},
    prelude::*,
    render::render_resource::PrimitiveTopology,
};
use pet_core::behavior::State;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reaction {
    Squeeze,
    Joy,
    Surprise,
    Wink,
}
impl Reaction {
    pub const ALL: [Self; 4] = [Self::Squeeze, Self::Joy, Self::Surprise, Self::Wink];
    pub fn for_click(serial: u64) -> Self {
        Self::ALL[serial.max(1).wrapping_sub(1) as usize % Self::ALL.len()]
    }
}
pub fn weight(a: &Avatar) -> f32 {
    if a.state != State::Petted {
        return 0.;
    }
    let enter = (a.clock / 0.10).clamp(0., 1.);
    let exit = ((1.65 - a.clock) / 0.32).clamp(0., 1.);
    let t = enter.min(exit);
    t * t * (3. - 2. * t)
}

#[derive(Component)]
pub struct FacePart {
    owner: Entity,
    reaction: Reaction,
    colour: [f32; 3],
    material: Handle<StandardMaterial>,
}

struct Surface(Vec<[Vec3; 3]>);
impl Surface {
    fn point(&self, p: Vec2, offset: f32) -> [f32; 3] {
        // Intersect the actual model's front surface rather than floating a flat decal.
        let z = self
            .0
            .iter()
            .filter_map(|[a, b, c]| {
                let v0 = b.truncate() - a.truncate();
                let v1 = c.truncate() - a.truncate();
                let v2 = p - a.truncate();
                let d = v0.perp_dot(v1);
                if d.abs() < 1e-8 {
                    return None;
                }
                let u = v2.perp_dot(v1) / d;
                let v = v0.perp_dot(v2) / d;
                (u >= -0.001 && v >= -0.001 && u + v <= 1.001)
                    .then_some(a.z + u * (b.z - a.z) + v * (c.z - a.z))
            })
            .reduce(f32::max)
            .unwrap_or(0.7);
        [p.x, p.y, z + offset]
    }
}
#[derive(Default)]
struct Drawing {
    points: Vec<Vec2>,
    indices: Vec<u32>,
}
impl Drawing {
    fn patch(&mut self, outline: &[Vec2]) {
        let base = self.points.len() as u32;
        self.points
            .push(outline.iter().copied().sum::<Vec2>() / outline.len() as f32);
        self.points.extend_from_slice(outline);
        for i in 0..outline.len() as u32 {
            self.indices.extend([
                base,
                base + 1 + i,
                base + 1 + (i + 1) % outline.len() as u32,
            ]);
        }
    }
    fn oval(&mut self, centre: Vec2, radius: Vec2) {
        let points = (0..40)
            .map(|i| {
                let t = i as f32 / 40. * std::f32::consts::TAU;
                centre + Vec2::new(t.cos(), t.sin()) * radius
            })
            .collect::<Vec<_>>();
        self.patch(&points);
    }
    fn line(&mut self, points: &[Vec2], radius: f32) {
        for p in points {
            self.oval(*p, Vec2::splat(radius));
        }
        for edge in points.windows(2) {
            let normal = (edge[1] - edge[0]).normalize_or_zero().perp() * radius;
            self.patch(&[
                edge[0] + normal,
                edge[1] + normal,
                edge[1] - normal,
                edge[0] - normal,
            ]);
        }
    }
    fn mesh(self, surface: &Surface, offset: f32) -> Mesh {
        let count = self.points.len();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            self.points
                .into_iter()
                .map(|p| surface.point(p, offset))
                .collect::<Vec<_>>(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0., 0., 1.]; count]);
        mesh.insert_indices(Indices::U32(self.indices));
        mesh
    }
}

fn draw(reaction: Reaction) -> [Drawing; 3] {
    let mut ink = Drawing::default();
    let mut pink = Drawing::default();
    let mut blush = Drawing::default();
    for side in [-1., 1.] {
        let centre = Vec2::new(side * 0.425, -0.32);
        if reaction == Reaction::Surprise || reaction == Reaction::Wink && side < 0. {
            ink.oval(centre, Vec2::new(0.065, 0.11));
        } else {
            let points = if reaction == Reaction::Squeeze || reaction == Reaction::Wink {
                vec![
                    Vec2::new(side * 0.16, 0.11),
                    Vec2::new(-side * 0.09, 0.),
                    Vec2::new(side * 0.16, -0.11),
                ]
            } else {
                (0..17)
                    .map(|i| {
                        let t = i as f32 / 16.;
                        Vec2::new(
                            (t - 0.5) * 0.32,
                            (t * std::f32::consts::PI).sin() * 0.09 - 0.025,
                        )
                    })
                    .collect()
            };
            ink.line(
                &points.iter().map(|p| centre + *p).collect::<Vec<_>>(),
                0.025,
            );
        }
        blush.oval(Vec2::new(side * 0.64, -0.53), Vec2::new(0.09, 0.04));
    }
    if reaction == Reaction::Surprise {
        ink.oval(Vec2::new(0., -0.64), Vec2::new(0.105, 0.12));
        pink.oval(Vec2::new(0., -0.65), Vec2::new(0.065, 0.078));
    } else {
        // Two soft lobes and a pinched centre echo the reference's excited open mouth.
        let outline = (0..64)
            .map(|i| {
                let t = i as f32 / 64. * std::f32::consts::TAU;
                Vec2::new(
                    t.cos() * 0.23,
                    -0.63 + t.sin() * (0.09 + 0.055 * t.cos().abs()),
                )
            })
            .collect::<Vec<_>>();
        ink.patch(&outline);
        pink.patch(
            &outline
                .iter()
                .map(|p| Vec2::new(p.x * 0.77, -0.63 + (p.y + 0.63) * 0.65))
                .collect::<Vec<_>>(),
        );
    }
    [ink, pink, blush]
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn install(
    commands: &mut Commands,
    owner: Entity,
    preview: bool,
    descendants: &[Entity],
    spatial: &Query<(&GlobalTransform, Option<&Mesh3d>, Option<&Name>)>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let Some(body) = descendants.iter().copied().find(|e| {
        spatial
            .get(*e)
            .is_ok_and(|(_, _, n)| n.is_some_and(|n| n.as_str() == "PlushBody"))
    }) else {
        return;
    };
    let inverse = spatial.get(body).unwrap().0.affine().inverse();
    let shell = descendants.iter().find_map(|e| {
        let (global, mesh, name) = spatial.get(*e).ok()?;
        let name = name?;
        if !name.as_str().starts_with("BodyShell") {
            return None;
        }
        Some((global, mesh?))
    });
    let Some((global, handle)) = shell else {
        return;
    };
    let Some(mesh) = meshes.get(&handle.0) else {
        return;
    };
    let Some(VertexAttributeValues::Float32x3(points)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return;
    };
    let Some(indices) = mesh.indices() else {
        return;
    };
    let indices = indices.iter().collect::<Vec<_>>();
    let relative = inverse * global.affine();
    let surface = Surface(
        indices
            .chunks_exact(3)
            .map(|tri| {
                std::array::from_fn(|i| relative.transform_point3(Vec3::from(points[tri[i]])))
            })
            .filter(|tri: &[Vec3; 3]| tri.iter().any(|p| p.z > 0.3))
            .collect(),
    );
    for reaction in Reaction::ALL {
        for (i, drawing) in draw(reaction).into_iter().enumerate() {
            let colour = [
                [0.065, 0.045, 0.055],
                [0.96, 0.48, 0.56],
                [0.94, 0.62, 0.61],
            ][i];
            let material = materials.add(StandardMaterial {
                base_color: Color::srgba(colour[0], colour[1], colour[2], 0.),
                unlit: true,
                alpha_mode: AlphaMode::Blend,
                cull_mode: None,
                ..default()
            });
            commands.spawn((
                FacePart {
                    owner,
                    reaction,
                    colour,
                    material: material.clone(),
                },
                Mesh3d(meshes.add(drawing.mesh(&surface, 0.035 + i as f32 * 0.006))),
                MeshMaterial3d(material),
                Transform::default(),
                Visibility::Hidden,
                RenderLayers::layer(if preview { 2 } else { 1 }),
                ChildOf(body),
            ));
        }
    }
}

pub fn update(
    avatars: Query<&Avatar>,
    mut parts: Query<(&FacePart, &mut Visibility)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (part, mut visible) in &mut parts {
        let amount = avatars
            .get(part.owner)
            .ok()
            .filter(|a| a.ready && Reaction::for_click(a.reaction) == part.reaction)
            .map_or(0., weight);
        *visible = if amount > 0.001 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if let Some(material) = materials.get_mut(&part.material) {
            material.base_color =
                Color::srgba(part.colour[0], part.colour[1], part.colour[2], amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reactions_differ_and_stay_on_front_surface() {
        let surface = Surface(vec![
            [
                Vec3::new(-1., -1., 0.8),
                Vec3::new(1., -1., 0.8),
                Vec3::new(1., 1., 0.8),
            ],
            [
                Vec3::new(-1., -1., 0.8),
                Vec3::new(1., 1., 0.8),
                Vec3::new(-1., 1., 0.8),
            ],
        ]);
        assert_eq!(surface.point(Vec2::ZERO, 0.03), [0., 0., 0.83]);
        let counts = Reaction::ALL.map(|r| {
            let drawings = draw(r);
            assert!(drawings.iter().all(|d| !d.indices.is_empty()));
            let count = drawings[0].points.len();
            for d in drawings {
                assert!(
                    d.points
                        .iter()
                        .all(|p| p.is_finite() && p.x.abs() < 0.9 && p.y > -0.9 && p.y < 0.)
                );
                assert!(d.indices.iter().all(|i| (*i as usize) < d.points.len()));
            }
            count
        });
        for pair in counts.windows(2) {
            assert_ne!(pair[0], pair[1]);
        }
        assert_eq!(Reaction::for_click(1), Reaction::Squeeze);
        assert_eq!(Reaction::for_click(5), Reaction::Squeeze);
    }
}
