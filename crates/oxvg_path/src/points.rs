//! Types used for processing polygons
use core::f64;

use crate::{
    command::{Data, ID},
    convert::{self, filter, to_absolute},
    geometry, positioned, Path,
};

#[derive(Default, Clone, Debug, PartialEq)]
/// The bounding points (min & max) for a list of points belonging to a path
pub struct Points {
    /// The list of points for each command in the path
    pub list: Vec<Point>,
    /// The x value of the minimum point
    pub min_x: f64,
    /// The y value of the minimum point
    pub min_y: f64,
    /// The x value of the maximum point
    pub max_x: f64,
    /// The y value of the maximum point
    pub max_y: f64,
}

#[derive(Default, Clone, Debug, PartialEq)]
/// The bounding points (min & max) for a list of points belonging to a command's movement
pub struct Point {
    /// The list of points for each point in the command
    pub list: Vec<geometry::Point>,
    /// The x value of the minimum point
    pub min_x: usize,
    /// The y value of the minimum point
    pub min_y: usize,
    /// The x value of the maximum point
    pub max_x: usize,
    /// The y value of the maximum point
    pub max_y: usize,
}

impl Points {
    /// Creates the list of points from a path.
    pub fn from_path(path: &Path) -> Self {
        Self::from_positioned(&convert::relative(path))
    }

    #[allow(clippy::too_many_lines)]
    /// Creates the list of points from a positioned path.
    pub fn from_positioned(path: &positioned::Path) -> Self {
        let mut points = Self::default();
        let mut prev_ctrl_point = [0.0; 2];

        let mut add_point = |path: &mut Point, point: [f64; 2]| {
            if path.list.is_empty() || point[1] > path.list[path.max_y].0[1] {
                path.max_y = path.list.len();
                points.max_y = f64::max(point[1], points.max_y);
            }
            if path.list.is_empty() || point[0] > path.list[path.max_x].0[0] {
                path.max_x = path.list.len();
                points.max_x = f64::max(point[0], points.max_x);
            }
            if path.list.is_empty() || point[1] < path.list[path.min_y].0[1] {
                path.min_y = path.list.len();
                points.min_y = f64::min(point[1], points.min_y);
            }
            if path.list.is_empty() || point[0] < path.list[path.min_x].0[0] {
                path.min_x = path.list.len();
                points.min_x = f64::min(point[0], points.min_x);
            }
            path.list.push(geometry::Point(point));
        };

        // chunked into move (e.g. `m0 0v10 m10 10v-10` -> [`m0 0v10`, `m10 10v-10`])
        for chunk in path
            .0
            .chunk_by(|_, p| !matches!(p.command.as_explicit().id(), ID::MoveTo | ID::MoveBy))
        {
            let sub_path =
                chunk
                    .iter()
                    .enumerate()
                    .fold(Point::default(), |mut sub_path, (i, p)| {
                        let c = to_absolute(p);
                        match c {
                            Data::MoveTo(data) => {
                                let mut sub_path = Point::default();
                                add_point(&mut sub_path, data);
                                return sub_path;
                            }
                            Data::HorizontalLineTo([x]) => {
                                let geometry::Point([_, y]) =
                                    sub_path.list.last().copied().unwrap_or_default();
                                add_point(&mut sub_path, [x, y]);
                            }
                            Data::VerticalLineTo([y]) => {
                                let geometry::Point([x, _]) =
                                    sub_path.list.last().copied().unwrap_or_default();
                                add_point(&mut sub_path, [x, y]);
                            }
                            Data::QuadraticBezierTo(data) => {
                                add_point(&mut sub_path, [data[0], data[1]]);
                                add_point(&mut sub_path, [data[2], data[3]]);
                                prev_ctrl_point = [data[2] - data[0], data[3] - data[1]];
                            }
                            Data::SmoothQuadraticBezierTo(data) => {
                                let geometry::Point(base_point) =
                                    sub_path.list.last().copied().unwrap_or_default();
                                if let Some(prev) = chunk.get(i - 1) {
                                    let ctrl_point = if matches!(
                                        convert::to_absolute(prev),
                                        Data::QuadraticBezierTo(_)
                                            | Data::SmoothQuadraticBezierTo(_)
                                    ) {
                                        [
                                            base_point[0] + prev_ctrl_point[0],
                                            base_point[1] + prev_ctrl_point[1],
                                        ]
                                    } else {
                                        [base_point[0], base_point[1]]
                                    };
                                    add_point(&mut sub_path, ctrl_point);
                                    prev_ctrl_point =
                                        [data[0] - ctrl_point[0], data[1] - ctrl_point[1]];
                                    add_point(&mut sub_path, data);
                                }
                            }
                            Data::CubicBezierTo(data) => {
                                if let Some(geometry::Point(base_point)) =
                                    sub_path.list.last().copied()
                                {
                                    add_point(
                                        &mut sub_path,
                                        [
                                            (base_point[0] + data[0]) / 2.0,
                                            (base_point[1] + data[1]) / 2.0,
                                        ],
                                    );
                                }
                                add_point(
                                    &mut sub_path,
                                    [(data[0] + data[2]) / 2.0, (data[1] + data[3]) / 2.0],
                                );
                                add_point(
                                    &mut sub_path,
                                    [(data[2] + data[4]) / 2.0, (data[3] + data[5]) / 2.0],
                                );
                                prev_ctrl_point = [data[4] - data[2], data[5] - data[3]];
                                add_point(&mut sub_path, [data[4], data[5]]);
                            }
                            Data::SmoothBezierTo(data) => {
                                let mut ctrl_point = sub_path.list.last().copied();
                                if let Some(geometry::Point(base_point)) = ctrl_point {
                                    if let Some(prev) = chunk.get(i - 1) {
                                        if matches!(
                                            convert::to_absolute(prev),
                                            Data::CubicBezierTo(_)
                                                | Data::SmoothQuadraticBezierTo(_)
                                        ) {
                                            add_point(
                                                &mut sub_path,
                                                [
                                                    base_point[0] + 0.5 * prev_ctrl_point[0],
                                                    base_point[1] + 0.5 * prev_ctrl_point[1],
                                                ],
                                            );
                                            ctrl_point = Some(geometry::Point([
                                                base_point[0] + prev_ctrl_point[0],
                                                base_point[1] + prev_ctrl_point[1],
                                            ]));
                                        }
                                    }
                                }
                                if let Some(geometry::Point(ctrl_point)) = ctrl_point {
                                    add_point(
                                        &mut sub_path,
                                        [
                                            0.5 * (ctrl_point[0] + data[0]),
                                            0.5 * (ctrl_point[1] + data[1]),
                                        ],
                                    );
                                }
                                add_point(
                                    &mut sub_path,
                                    [0.5 * (data[0] + data[2]), 0.5 * (data[1] + data[3])],
                                );
                                add_point(&mut sub_path, [data[2], data[3]]);
                                prev_ctrl_point = [data[2] - data[0], data[3] - data[1]];
                            }
                            Data::ArcTo(data) => {
                                let base_point = sub_path.list.last().copied();
                                if let Some(geometry::Point(base_point_inner)) = base_point {
                                    let curves =
                                        filter::arc::Convert::a2c(&base_point_inner, &data, None);
                                    let end = curves.len() / 6;
                                    let mut prev_base_point = base_point_inner;
                                    for (i, c_data) in curves.chunks(6).enumerate() {
                                        add_point(
                                            &mut sub_path,
                                            [
                                                prev_base_point[0] + (c_data[0] / 2.0),
                                                prev_base_point[1] + (c_data[1] / 2.0),
                                            ],
                                        );
                                        add_point(
                                            &mut sub_path,
                                            [
                                                prev_base_point[0] + (c_data[0] + c_data[2]) / 2.0,
                                                prev_base_point[1] + (c_data[1] + c_data[3]) / 2.0,
                                            ],
                                        );
                                        add_point(
                                            &mut sub_path,
                                            [
                                                prev_base_point[0] + (c_data[2] + c_data[4]) / 2.0,
                                                prev_base_point[1] + (c_data[3] + c_data[5]) / 2.0,
                                            ],
                                        );
                                        if i < end - 1 {
                                            prev_ctrl_point = [
                                                prev_base_point[0] + c_data[4],
                                                prev_base_point[1] + c_data[5],
                                            ];
                                            prev_base_point = prev_ctrl_point;
                                            add_point(&mut sub_path, prev_ctrl_point);
                                        }
                                    }
                                    add_point(&mut sub_path, [data[5], data[6]]);
                                }
                            }
                            Data::LineTo(data) => add_point(&mut sub_path, data),
                            Data::ClosePath => {}
                            _ => unreachable!("found unreachable command {c:?}"),
                        }
                        sub_path
                    });

            points.list.push(sub_path);
        }

        points
    }
}

impl Point {
    /// Forms a convex hull from set of points of every subpath using monotone chain convex hull
    /// algorithm.
    pub fn convex_hull(&self) -> Self {
        let mut list = self.list.clone();
        list.sort_by(|geometry::Point(a), geometry::Point(b)| {
            if a[0] == b[0] {
                a[1].total_cmp(&b[1])
            } else {
                a[0].total_cmp(&b[0])
            }
        });

        let mut lower = vec![];
        let mut min_y = 0;
        let mut bottom = 0;

        for (i, point) in list.iter().enumerate() {
            while lower.len() >= 2
                && geometry::Point::cross(lower[lower.len() - 2], lower[lower.len() - 1], point)
                    <= 0.0
            {
                lower.pop();
            }
            if point.0[1] < list[min_y].0[1] {
                min_y = i;
                bottom = lower.len();
            }
            lower.push(*point);
        }

        let mut upper = vec![];
        let mut max_y = self.list.len() - 1;
        let mut top = 0;

        for (i, point) in list.iter().enumerate().rev() {
            while upper.len() >= 2
                && geometry::Point::cross(upper[upper.len() - 2], upper[upper.len() - 1], point)
                    <= 0.0
            {
                upper.pop();
            }
            if point.0[1] > list[max_y].0[1] {
                max_y = i;
                top = upper.len();
            }
            upper.push(*point);
        }

        upper.pop();
        lower.pop();

        let max_x = lower.len();
        lower.extend(upper);
        let max_y = if lower.is_empty() {
            0
        } else {
            (max_x + top) % lower.len()
        };

        Self {
            list: lower,
            min_x: 0,
            max_x,
            min_y: bottom,
            max_y,
        }
    }

    /// Gets the support point of the Minowski difference of two shapes.
    pub fn get_support(&self, other: &Point, direction: geometry::Point) -> geometry::Point {
        self.support_point(direction)
            .sub(other.support_point(direction.minus()))
    }

    /// Get the supporting point of a polygon, the furthest point in a given direction.
    pub fn support_point(&self, geometry::Point(direction): geometry::Point) -> geometry::Point {
        let mut index = if direction[1] >= 0.0 {
            if direction[0] < 0.0 {
                self.max_y
            } else {
                self.max_x
            }
        } else if direction[0] < 0.0 {
            self.min_x
        } else {
            self.min_y
        };
        let mut max = -f64::INFINITY;
        loop {
            let value = self.list[index].dot(&geometry::Point(direction));
            if value <= max {
                break;
            }
            max = value;
            index = (index + 1) % self.list.len();
        }

        if index == 0 {
            index = self.list.len();
        }
        self.list[index - 1]
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn from_positioned() {
    let path = convert::relative(&Path::parse_string("m10 10 m 10 10").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![
                    Point {
                        list: vec![geometry::Point([10.0, 10.0])],
                        min_x: 0,
                        min_y: 0,
                        max_x: 0,
                        max_y: 0,
                    },
                    Point {
                        list: vec![geometry::Point([20.0, 20.0])],
                        min_x: 0,
                        min_y: 0,
                        max_x: 0,
                        max_y: 0,
                    }
                ],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 20.0,
                max_y: 20.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 l 10 10").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![geometry::Point([10.0, 10.0]), geometry::Point([20.0, 20.0])],
                    min_x: 0,
                    min_y: 0,
                    max_x: 1,
                    max_y: 1,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 20.0,
                max_y: 20.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 h 10").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![geometry::Point([10.0, 10.0]), geometry::Point([20.0, 10.0])],
                    min_x: 0,
                    min_y: 0,
                    max_x: 1,
                    max_y: 0,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 20.0,
                max_y: 10.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 v 10").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![geometry::Point([10.0, 10.0]), geometry::Point([10.0, 20.0])],
                    min_x: 0,
                    min_y: 0,
                    max_x: 0,
                    max_y: 1,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 10.0,
                max_y: 20.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 c20 0 15 -80 40 -80").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([20.0, 10.0]),
                        geometry::Point([27.5, -30.0]),
                        geometry::Point([37.5, -70.0]),
                        geometry::Point([50.0, -70.0])
                    ],
                    min_x: 0,
                    min_y: 3,
                    max_x: 4,
                    max_y: 0,
                },],
                min_x: 0.0,
                min_y: -70.0,
                max_x: 50.0,
                max_y: 10.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 s20 80 40 80").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([20.0, 50.0]),
                        geometry::Point([40.0, 90.0]),
                        geometry::Point([50.0, 90.0]),
                    ],
                    min_x: 0,
                    min_y: 0,
                    max_x: 3,
                    max_y: 2,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 50.0,
                max_y: 90.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 c20 0 15 -80 40 -80").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([20.0, 10.0]),
                        geometry::Point([27.5, -30.0]),
                        geometry::Point([37.5, -70.0]),
                        geometry::Point([50.0, -70.0]),
                    ],
                    min_x: 0,
                    min_y: 3,
                    max_x: 4,
                    max_y: 0,
                },],
                min_x: 0.0,
                min_y: -70.0,
                max_x: 50.0,
                max_y: 10.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 c20 0 15 -80 40 -80 s20 80 40 80").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([20.0, 10.0]),
                        geometry::Point([27.5, -30.0]),
                        geometry::Point([37.5, -70.0]),
                        geometry::Point([50.0, -70.0]),
                        geometry::Point([62.5, -70.0]),
                        geometry::Point([72.5, -30.0]),
                        geometry::Point([80.0, 10.0]),
                        geometry::Point([90.0, 10.0]),
                    ],
                    min_x: 0,
                    min_y: 3,
                    max_x: 8,
                    max_y: 0,
                },],
                min_x: 0.0,
                min_y: -70.0,
                max_x: 90.0,
                max_y: 10.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 q25 25 40 50").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([35.0, 35.0]),
                        geometry::Point([50.0, 60.0]),
                    ],
                    min_x: 0,
                    min_y: 0,
                    max_x: 2,
                    max_y: 2,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 50.0,
                max_y: 60.0,
            }
        )
    );

    let path =
        convert::relative(&Path::parse_string("m10 10 q25 25 40 50 t30 0 30 0 30 0 30 0 30 0").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([35.0, 35.0]),
                        geometry::Point([50.0, 60.0]),
                        geometry::Point([65.0, 85.0]),
                        geometry::Point([80.0, 60.0]),
                        geometry::Point([95.0, 35.0]),
                        geometry::Point([110.0, 60.0]),
                        geometry::Point([125.0, 85.0]),
                        geometry::Point([140.0, 60.0]),
                        geometry::Point([155.0, 35.0]),
                        geometry::Point([170.0, 60.0]),
                        geometry::Point([185.0, 85.0]),
                        geometry::Point([200.0, 60.0]),
                    ],
                    min_x: 0,
                    min_y: 0,
                    max_x: 12,
                    max_y: 3,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 200.0,
                max_y: 85.0,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 a 6 4 10 1 0 14 10").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:.12?}"),
        format!(
            "{:.12?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([7.620_792_129_809_95, 11.602_909_896_518_065]),
                        geometry::Point([6.680_818_445_260_106_5, 16.236_683_612_154_636]),
                        geometry::Point([11.650_647_849_602_528, 20.089_328_590_049_874]),
                        geometry::Point([15.181_243_068_304_743, 20.911_109_748_826_61]),
                        geometry::Point([16.819_798_064_362_146, 21.292_499_724_161_825]),
                        geometry::Point([20.147_956_269_888_002, 21.450_566_227_695_43]),
                        geometry::Point([22.941_753_689_828_488, 20.483_329_632_134_907]),
                        geometry::Point([24.0, 20.0])
                    ],
                    min_x: 2,
                    min_y: 0,
                    max_x: 8,
                    max_y: 6,
                },],
                min_x: 0.0,
                min_y: 0.0,
                max_x: 24.0,
                max_y: 21.450_566_227_695_43,
            }
        )
    );

    let path = convert::relative(&Path::parse_string("m10 10 l10 10 h10 v10 c20 0 15 -80 40 -80 s20 80 40 80 q25 25 40 50 t30 0 30 0 30 0 30 0 30 0 a 6 4 10 1 0 14 10 z").unwrap());
    let points = Points::from_positioned(&path);
    pretty_assertions::assert_eq!(
        format!("{points:#?}"),
        format!(
            "{:#?}",
            Points {
                list: vec![Point {
                    list: vec![
                        geometry::Point([10.0, 10.0]),
                        geometry::Point([20.0, 20.0]),
                        geometry::Point([30.0, 20.0]),
                        geometry::Point([30.0, 30.0]),
                        geometry::Point([40.0, 30.0]),
                        geometry::Point([47.5, -10.0]),
                        geometry::Point([57.5, -50.0]),
                        geometry::Point([70.0, -50.0]),
                        geometry::Point([82.5, -50.0]),
                        geometry::Point([92.5, -10.0]),
                        geometry::Point([100.0, 30.0]),
                        geometry::Point([110.0, 30.0]),
                        geometry::Point([135.0, 55.0]),
                        geometry::Point([150.0, 80.0]),
                        geometry::Point([165.0, 105.0]),
                        geometry::Point([180.0, 80.0]),
                        geometry::Point([195.0, 55.0]),
                        geometry::Point([210.0, 80.0]),
                        geometry::Point([225.0, 105.0]),
                        geometry::Point([240.0, 80.0]),
                        geometry::Point([255.0, 55.0]),
                        geometry::Point([270.0, 80.0]),
                        geometry::Point([285.0, 105.0]),
                        geometry::Point([300.0, 80.0]),
                        geometry::Point([297.620_792_142_532_37, 81.602_909_905_339_74]),
                        geometry::Point([296.680_818_490_778_34, 86.236_683_627_144_79]),
                        geometry::Point([301.650_647_918_530_64, 90.089_328_591_442_58]),
                        geometry::Point([305.181_243_140_569_2, 90.911_109_739_275_08]),
                        geometry::Point([306.819_798_122_751_25, 91.292_499_705_941_01]),
                        geometry::Point([310.147_956_296_634_2, 91.450_566_203_153_76]),
                        geometry::Point([312.941_753_694_317_87, 90.483_329_621_038_28]),
                        geometry::Point([314.0, 90.0])
                    ],
                    min_x: 0,
                    min_y: 6,
                    max_x: 31,
                    max_y: 14,
                }],
                min_x: 0.0,
                min_y: -50.0,
                max_x: 314.0,
                max_y: 105.0,
            }
        )
    );
}
