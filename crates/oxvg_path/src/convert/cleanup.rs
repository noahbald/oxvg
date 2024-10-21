use crate::{
    command::{self, Position},
    geometry::Point,
    positioned::Path,
};

pub fn cleanup_unpositioned(path: &crate::Path) -> crate::Path {
    let path = Path(
        path.0
            .iter()
            .map(|p| Position {
                command: p.clone(),
                start: Point([0.0; 2]),
                end: Point([0.0; 2]),
                s_data: None,
            })
            .collect(),
    );
    cleanup(&path).take()
}

pub fn cleanup(path: &Path) -> Path {
    let mut result = remove_repeated_moves(path);
    switch_leading_move(&mut result);
    let mut result = ensure_implicity(&mut result);
    if result.0.len() == 1 {
        if let command::Data::MoveBy(a) = result.0[0].command {
            result.0[0].command = command::Data::MoveTo(a);
        }
    }
    #[cfg(debug_assertions)]
    {
        let path_dbg = path.clone().take().to_string();
        let result_dbg = result.clone().take().to_string();
        if path_dbg != result_dbg {
            log::debug!("convert::mixed: updated path: {result_dbg}");
        }
    }
    result
}

fn remove_repeated_moves(path: &Path) -> Path {
    let mut new_path: Vec<_> = path.0.clone().into_iter().map(Some).collect();
    (0..new_path.len()).for_each(|index| {
        let Some((prev_option, item_option, _)) =
            Path::split_mut_with_prev_option(&mut new_path, index)
        else {
            return;
        };

        let item = item_option
            .as_mut()
            .expect("`split_mut` guard would have returned if item is `None`");
        if matches!(
            item.command,
            command::Data::MoveBy(_) | command::Data::MoveTo(_)
        ) && matches!(
            prev_option.as_ref().map(|p| &p.command),
            Some(command::Data::MoveBy(_) | command::Data::MoveTo(_))
        ) {
            match prev_option {
                Some(Position {
                    command: command::Data::MoveBy(prev_a),
                    ..
                }) => match item.command {
                    command::Data::MoveBy(a) => {
                        *prev_a = [prev_a[0] + a[0], prev_a[1] + a[1]];
                        *item_option = None;
                    }
                    command::Data::MoveTo(_) => {
                        *prev_a = [
                            prev_a[0] + item.end.0[0] - item.start.0[0],
                            prev_a[1] + item.end.0[1] - item.start.0[1],
                        ];
                        *item_option = None;
                    }
                    _ => {}
                },
                Some(Position {
                    command: command::Data::MoveTo(prev_a),
                    ..
                }) => match item.command {
                    command::Data::MoveBy(_) | command::Data::MoveTo(_) => {
                        *prev_a = item.end.0;
                        *item_option = None;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    });
    Path(new_path.into_iter().flatten().collect())
}

fn switch_leading_move(path: &mut Path) {
    let Some((first, rest)) = path.0.split_first_mut() else {
        return;
    };
    let Some((second, _)) = rest.split_first_mut() else {
        return;
    };
    match second {
        Position { command: c, .. } if matches!(c, command::Data::LineTo(_)) => {
            if let Position {
                command: command::Data::MoveBy(a),
                ..
            } = first
            {
                first.command = command::Data::MoveTo(*a);
                second.command = command::Data::Implicit(Box::new(c.clone()));
            }
        }
        Position { command: c, .. } if matches!(c, command::Data::LineBy(_)) => {
            if let Position {
                command: command::Data::MoveTo(a),
                ..
            } = first
            {
                first.command = command::Data::MoveBy(*a);
                second.command = command::Data::Implicit(Box::new(c.clone()));
            }
        }
        _ => {
            if let Position {
                command: command::Data::MoveBy(a),
                ..
            } = first
            {
                first.command = command::Data::MoveTo(*a);
            }
        }
    }
}

fn ensure_implicity(path: &mut Path) -> Path {
    // TODO: Fix emplicity corrections elsewhere
    let mut new_path: Vec<_> = path.0.clone().into_iter().map(Some).collect();
    (0..new_path.len()).for_each(|index| {
        let Some((prev, item_option, _)) = Path::split_mut(&mut new_path, index) else {
            return;
        };

        let Some(item) = item_option else {
            return;
        };
        let prev_id = prev.command.id();
        let item_id = item.command.id();
        if !item_id.is_implicit() && prev_id.next_implicit() == item_id {
            item.command = command::Data::Implicit(Box::new(item.command.clone()));
        } else if item_id.is_implicit() && &prev_id.next_implicit() != item_id.as_explicit() {
            item.command = item.command.as_explicit().clone();
        }
    });
    Path(new_path.into_iter().flatten().collect())
}
