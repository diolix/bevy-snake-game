use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer, window::WindowResolution};
use rand::prelude::random;

const SNAKE_HEAD_COLOR: Color = Color::rgb(0.7, 0.7, 0.7);
const FOOD_COLOR: Color = Color::rgb(1.0, 0.0, 1.0);
const SNAKE_SEGMENT_COLOR: Color = Color::rgb(0.3, 0.3, 0.3);
const ARENA_WIDTH: u32 = 10;
const ARENA_HEIGHT: u32 = 10;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.04, 0.04, 0.04)))
        .insert_resource(LastTailPosition(std::option::Option::None))
        .insert_resource(SnakeSegments::default())
        .add_systems(Startup, (setup_camera, spawn_snake))
        .add_systems(
            Update,
            (
                position_translation,
                size_scaling,
                food_spawner.run_if(on_timer(Duration::from_secs(1))),
                (
                    snake_movement_input,
                    snake_movement.run_if(on_timer(Duration::from_millis(150))),
                    game_over,
                )
                    .chain(),
                (snake_eating, snake_growth).chain(),
            ),
        )
        .add_event::<GrowthEvent>()
        .add_event::<GameOverEvent>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(500.0, 500.0).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }))
        .run();
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct SnakeHead {
    direction: Direction,
}

#[derive(PartialEq, Copy, Clone)]
enum Direction {
    Left,
    Up,
    Right,
    Down,
}

impl Direction {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }
}

#[derive(Component)]
struct SnakeSegment;

#[derive(Default, Resource)]
struct SnakeSegments(Vec<Entity>);

#[derive(Default, Resource)]
struct LastTailPosition(Option<Position>);

#[derive(Component)]
struct Food;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Component)]
struct Size {
    width: f32,
    height: f32,
}

#[derive(Event)]
struct GrowthEvent;

#[derive(Event)]
struct GameOverEvent;

impl Size {
    pub fn square(x: f32) -> Self {
        Self {
            width: x,
            height: x,
        }
    }
}

fn spawn_snake(mut commands: Commands, mut segments: ResMut<SnakeSegments>) {
    *segments = SnakeSegments(vec![
        spawn_head(&mut commands),
        spawn_segment(commands, Position { x: 3, y: 2 }),
    ]);
}

fn food_spawner(mut commands: Commands) {
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: FOOD_COLOR,
                ..default()
            },
            ..default()
        },
        Food,
        Position {
            x: (random::<f32>() * ARENA_WIDTH as f32) as i32,
            y: (random::<f32>() * ARENA_HEIGHT as f32) as i32,
        },
        Size::square(0.8),
    ));
}

fn snake_movement(
    segments: ResMut<SnakeSegments>,
    mut last_tail_position: ResMut<LastTailPosition>,
    head_querry: Query<(Entity, &SnakeHead)>,
    mut game_over_writer: EventWriter<GameOverEvent>,
    mut positions_querry: Query<&mut Position>,
) {
    let (head_entity, head) = match head_querry.get_single() {
        Ok((head_entity_ok, head_ok)) => (head_entity_ok, head_ok),
        Err(error) => {
            println!("snake_movement, error trying to access head : {:?}", error);
            return;
        }
    };
    let segment_positions = segments
        .0
        .iter()
        .map(|e| *positions_querry.get_mut(*e).unwrap())
        .collect::<Vec<Position>>();

    let mut head_position = positions_querry.get_mut(head_entity).unwrap();

    match &head.direction {
        Direction::Left => head_position.x -= 1,
        Direction::Up => head_position.y += 1,
        Direction::Right => head_position.x += 1,
        Direction::Down => head_position.y -= 1,
    }

    if head_position.x < 0
        || head_position.y < 0
        || head_position.x as u32 >= ARENA_WIDTH
        || head_position.y as u32 >= ARENA_HEIGHT
    {
        game_over_writer.send(GameOverEvent);
    }

    if segment_positions.contains(&head_position) {
        game_over_writer.send(GameOverEvent);
    }

    segment_positions
        .iter()
        .zip(segments.0.iter().skip(1))
        .for_each(|(pos, segment)| {
            *positions_querry.get_mut(*segment).unwrap() = *pos;
        });

    *last_tail_position = LastTailPosition(Some(*segment_positions.last().unwrap()));
}

fn snake_movement_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query_head: Query<&mut SnakeHead>,
) {
    let mut head = match query_head.get_single_mut() {
        Ok(snake_head) => snake_head,
        Err(error) => {
            println!(
                "snake_movement_input, error trying to access head : {:?}",
                error
            );
            return;
        }
    };

    let direction_input: Direction = if keyboard_input.pressed(KeyCode::Left) {
        Direction::Left
    } else if keyboard_input.pressed(KeyCode::Down) {
        Direction::Down
    } else if keyboard_input.pressed(KeyCode::Up) {
        Direction::Up
    } else if keyboard_input.pressed(KeyCode::Right) {
        Direction::Right
    } else {
        head.direction
    };

    if direction_input != head.direction.opposite() {
        head.direction = direction_input;
    }
}

fn size_scaling(query_window: Query<&Window>, mut query_size: Query<(&Size, &mut Transform)>) {
    let window = query_window.single();
    for (sprite_size, mut transform) in query_size.iter_mut() {
        transform.scale = Vec3::new(
            sprite_size.width / ARENA_WIDTH as f32 * window.width(),
            sprite_size.height / ARENA_HEIGHT as f32 * window.height(),
            1.0,
        );
    }
}

fn position_translation(
    query_window: Query<&Window>,
    mut query_position: Query<(&Position, &mut Transform)>,
) {
    fn convert(pos: f32, bound_window: f32, bound_game: f32) -> f32 {
        let tile_size = bound_window / bound_game;
        pos / bound_game * bound_window - (bound_window / 2.) + (tile_size / 2.)
    }
    let window = query_window.single();
    for (pos, mut transform) in query_position.iter_mut() {
        transform.translation = Vec3::new(
            convert(pos.x as f32, window.width() as f32, ARENA_WIDTH as f32),
            convert(pos.y as f32, window.height() as f32, ARENA_HEIGHT as f32),
            0.0,
        );
    }
}

fn spawn_segment(mut commands: Commands, position: Position) -> Entity {
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_SEGMENT_COLOR,
                    ..default()
                },
                ..default()
            },
            SnakeSegment,
            position,
            Size::square(0.65),
        ))
        .id()
}

fn spawn_head(commands: &mut Commands) -> Entity {
    commands
        .spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: SNAKE_HEAD_COLOR,
                    ..default()
                },
                transform: Transform {
                    scale: Vec3::new(10.0, 10.0, 10.0),
                    ..default()
                },
                ..default()
            },
            SnakeHead {
                direction: Direction::Up,
            },
            Position { x: 3, y: 3 },
            Size::square(0.8),
        ))
        .id()
}

fn snake_eating(
    mut commands: Commands,
    mut growth_writer: EventWriter<GrowthEvent>,
    food_positions: Query<(Entity, &Position), With<Food>>,
    head_positions: Query<&Position, With<SnakeHead>>,
) {
    for head_pos in head_positions.iter() {
        for (ent, food_pos) in food_positions.iter() {
            if food_pos == head_pos {
                commands.entity(ent).despawn();
                growth_writer.send(GrowthEvent);
            }
        }
    }
}

fn snake_growth(
    commands: Commands,
    last_tail_position: Res<LastTailPosition>,
    mut segments: ResMut<SnakeSegments>,
    mut growth_reader: EventReader<GrowthEvent>,
) {
    if growth_reader.read().next().is_some() {
        segments
            .0
            .push(spawn_segment(commands, last_tail_position.0.unwrap()));
    }
}

fn game_over(
    mut commands: Commands,
    mut reader: EventReader<GameOverEvent>,
    segments_res: ResMut<SnakeSegments>,
    foods: Query<Entity, With<Food>>,
) {
    if reader.read().next().is_none() {
        return;
    };

    for food in foods.iter() {
        commands.entity(food).despawn();
    }

    for segment in segments_res.0.iter() {
        commands.entity(*segment).despawn();
    }

    spawn_snake(commands, segments_res);
}
