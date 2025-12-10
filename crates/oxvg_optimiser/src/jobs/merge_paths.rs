use std::cell::{self, RefMut};

use itertools::Itertools as _;
use lightningcss::properties::{
    effects::{Filter, FilterList},
    svg::SVGPaint,
};
use oxvg_ast::{
    element::Element,
    get_attribute, get_attribute_mut, get_computed_style, has_attribute, has_computed_style,
    has_computed_style_css, is_attribute, is_element, set_attribute,
    style::{ComputedStyles, Mode},
    visitor::{Context, PrepareOutcome, Visitor},
};
use oxvg_collections::attribute::{inheritable::Inheritable, path};
use oxvg_path::command;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "wasm")]
use tsify::Tsify;

use crate::error::JobsError;

#[cfg_attr(feature = "wasm", derive(Tsify))]
#[cfg_attr(feature = "napi", napi(object))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
/// Merge multiple paths into one
///
/// # Differences to SVGO
///
/// There's no need to specify precision or spacing for path serialization.
///
/// # Correctness
///
/// By default this job should never visually change the document.
///
/// Running with `force` may cause intersecting paths to be incorrectly merged.
///
/// # Errors
///
/// Never.
///
/// If this job produces an error or panic, please raise an [issue](https://github.com/noahbald/oxvg/issues)
pub struct MergePaths {
    #[cfg_attr(feature = "serde", serde(default = "default_force"))]
    /// Whether to merge paths despite intersections
    pub force: bool,
}

impl Default for MergePaths {
    fn default() -> Self {
        MergePaths {
            force: default_force(),
        }
    }
}

impl<'input, 'arena> Visitor<'input, 'arena> for MergePaths {
    type Error = JobsError<'input>;

    fn prepare(
        &self,
        document: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<PrepareOutcome, Self::Error> {
        context.query_has_stylesheet(document);
        Ok(PrepareOutcome::none)
    }

    #[allow(clippy::too_many_lines)]
    fn element(
        &self,
        element: &Element<'input, 'arena>,
        context: &mut Context<'input, 'arena, '_>,
    ) -> Result<(), Self::Error> {
        let mut children = itertools::peek_nth(element.children_iter());
        if children.peek_nth(1).is_none() {
            return Ok(());
        }

        let mut prev_path_data: Option<path::Path> = None;

        for (prev_child, child) in children.tuple_windows() {
            log::debug!("trying to merge {child:?}");
            macro_rules! update_previous_path {
                ($prev_child:ident) => {
                    if let Some(data) = prev_path_data.take() {
                        set_attribute!(prev_child, D(data));
                    }
                    prev_path_data = None;
                };
            }

            if !is_element!(prev_child, Path)
                || !prev_child.is_empty()
                || has_attribute!(prev_child, Id)
            {
                log::debug!("ending merge, prev not a plain path");
                update_previous_path!(prev_child);
                continue;
            }

            if !is_element!(child, Path) || !child.is_empty() {
                log::debug!("ending merge, current not a plain path");
                update_previous_path!(prev_child);
                continue;
            }
            let computed_styles = ComputedStyles::default()
                .with_all(&child, &context.query_has_stylesheet_result)
                .map_err(JobsError::ComputedStylesError)?;
            let Some(mut current_path_data) =
                get_attribute_mut!(child, D).map(|d| RefMut::map(d, |path::Path(d, _)| d))
            else {
                log::debug!("ending merge, current has no `d`");
                update_previous_path!(prev_child);
                continue;
            };
            if let Some(first) = current_path_data.0.first_mut() {
                if let command::Data::MoveBy(data) = first {
                    *first = command::Data::MoveTo(*data);

                    if let Some(second) = current_path_data.0.get_mut(1) {
                        if second.is_implicit() && second.as_explicit().id() != command::ID::LineTo
                        {
                            *second = second.as_explicit().clone();
                        }
                    }
                }
            }
            drop(current_path_data);

            if
                has_computed_style!(
                    computed_styles,
                    MarkerStart | MarkerMid | MarkerEnd | ClipPath | Mask
                )
                || has_computed_style_css!(computed_styles, MaskImage(None))
                || get_computed_style!(computed_styles, Fill).is_some_and(|(fill, mode)| {
                    matches!(mode, Mode::Static)
                        && matches!(fill.option(), Some(SVGPaint::Url { url,.. }) if url.url.starts_with('#'))
                })
                || get_computed_style!(computed_styles, Filter).is_some_and(|(filter, mode)| {
                    matches!(mode, Mode::Static)
                        && matches!(filter, Inheritable::Defined(FilterList::Filters(filters)) if filters.iter().any(|filter| matches!(filter, Filter::Url(url) if url.url.starts_with('#'))))
                })
                || get_computed_style!(computed_styles, Stroke).is_some_and(|(stroke, mode)| {
                    matches!(mode, Mode::Static) && matches!(stroke.option(), Some(SVGPaint::Url { url,.. }) if url.url.starts_with('#'))
                })
            {
                log::debug!("ending merge, has forbidden style or reference");
                update_previous_path!(prev_child);
                continue;
            }

            let prev_attrs = prev_child.attributes();
            let attrs = child.attributes();
            if prev_attrs.len() != attrs.len() {
                log::debug!("ending merge, current attrs length different to prev");
                update_previous_path!(prev_child);
                continue;
            }

            let are_any_attr_diff = attrs.into_iter().any(|a| {
                !is_attribute!(a, D) && prev_attrs.get_named_item(a.name()).is_none_or(|p| *p != *a)
            });
            if are_any_attr_diff {
                log::debug!("ending merge, current attrs equal to prev");
                update_previous_path!(prev_child);
                continue;
            }

            let has_prev_path = prev_path_data.is_some();
            if prev_path_data.is_none() {
                prev_path_data = get_attribute!(prev_child, D).as_deref().cloned();
            }

            let current_path_data = get_attribute!(child, D)
                .map(|d| cell::Ref::map(d, |path::Path(d, _)| d))
                .expect("D previously used");
            if let Some(path::Path(prev_path_data, _)) = &mut prev_path_data {
                if prev_path_data.0.last().is_some_and(|d| {
                    matches!(
                        d.id().as_explicit(),
                        command::ID::MoveTo | command::ID::MoveBy
                    )
                }) {
                    prev_path_data.0.pop();
                }
                if self.force || !prev_path_data.intersects(&current_path_data) {
                    log::debug!("merging, current doesn't intersect prev");
                    prev_path_data.0.extend(current_path_data.0.clone());
                    prev_child.remove();
                    continue;
                }
            }

            log::debug!("ending merge, current doesn't intersect prev");
            if has_prev_path {
                update_previous_path!(prev_child);
            } else {
                prev_path_data = None;
            }
        }
        if let Some(prev_path_data) = prev_path_data {
            set_attribute!(element.last_element_child().unwrap(), D(prev_path_data));
        }

        Ok(())
    }
}

const fn default_force() -> bool {
    false
}

#[test]
#[allow(clippy::too_many_lines)]
fn merge_paths() -> anyhow::Result<()> {
    use crate::test_config;

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- merge paths without attributes -->
    <path d="M 0,0 z"/>
    <path d="M 10,10 z"/>
    <path d="M 20,20 l 10,10 M 30,0 c 10,0 20,10 20,20"/>
    <path d="M 30,30 z"/>
    <path d="M 30,30 z" fill="#f00"/>
    <path d="M 40,40 z"/>
    <path d="m 50,50 0,10 20,30 40,0"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- merge sequence of paths with same attributes -->
    <path d="M 0,0 z" fill="#fff" stroke="#333"/>
    <path d="M 10,10 z" fill="#fff" stroke="#333"/>
    <path d="M 20,20" fill="#fff" stroke="#333"/>
    <path d="M 30,30 z" fill="#fff" stroke="#333"/>
    <path d="M 30,30 z" fill="#f00"/>
    <path d="M 40,40 z"/>
    <path d="m 50,50 z"/>
    <path d="M 40,40"/>
    <path d="m 50,50"/>
    <path d="M 40,40 z" fill="#fff" stroke="#333"/>
    <path d="m 50,50 z" fill="#fff" stroke="#333"/>
    <path d="M 40,40" fill="#fff" stroke="#333"/>
    <path d="m 50,50" fill="#fff" stroke="#333"/>
    <path d="m 50,50 z" fill="#fff" stroke="#333"/>
    <path d="M0 0v100h100V0z" fill="red"/>
    <path d="M200 0v100h100V0z" fill="red"/>
    <path d="M0 0v100h100V0z" fill="blue"/>
    <path d="M200 0v100h100V0zM0 200h100v100H0z" fill="blue"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <!-- merge only intersected paths -->
    <path d="M30 0L0 40H60z"/>
    <path d="M0 10H60L30 50z"/>
    <path d="M0 0V50L50 0"/>
    <path d="M0 60L50 10V60"/>
    <g>
        <path d="M100 0a50 50 0 0 1 0 100"/>
        <path d="M25 25H75V75H25z"/>
        <path d="M135 85H185V135H135z"/>
    </g>
    <g>
        <path d="M10 14H7v1h3v-1z"/>
        <path d="M9 21H8v1h1v-1z"/>
    </g>
    <g>
        <path d="M30 32.705V40h10.42L30 32.705z"/>
        <path d="M46.25 34.928V30h-7.04l7.04 4.928z"/>
    </g>
    <g>
        <path d="M20 20H60L100 30"/>
        <path d="M20 20L50 30H100"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg">
    <path d="M320 60c17.466-8.733 33.76-12.78 46.593-12.484 12.856.297 22.254 4.936 26.612 12.484 4.358 7.548 3.676 18.007-2.494 29.29-6.16 11.26-17.812 23.348-34.107 34.107-16.26 10.735-37.164 20.14-60.72 26.613C272.356 156.473 246.178 160 220 160c-26.18 0-52.357-3.527-75.882-9.99-23.557-6.472-44.462-15.878-60.72-26.613-16.296-10.76-27.95-22.846-34.11-34.108-6.17-11.283-6.85-21.742-2.493-29.29 4.358-7.548 13.756-12.187 26.612-12.484C86.24 47.22 102.535 51.266 120 60c17.426 8.713 36.024 22.114 53.407 39.28C190.767 116.42 206.91 137.33 220 160c13.09 22.67 23.124 47.106 29.29 70.71 6.173 23.638 8.48 46.445 7.313 65.893-1.17 19.49-5.812 35.627-12.485 46.592C237.432 354.18 228.716 360 220 360s-17.432-5.82-24.118-16.805c-6.673-10.965-11.315-27.1-12.485-46.592-1.167-19.448 1.14-42.255 7.314-65.892 6.166-23.604 16.2-48.04 29.29-70.71 13.09-22.67 29.233-43.58 46.593-60.72C283.976 82.113 302.573 68.712 320 60z"/>
    <path d="M280 320l100-173.2h200l100 173.2-100 173.2h-200"/>
    <g>
        <path d="M706.69 299.29c-.764-11.43-6.036-56.734-16.338-71.32 0 0 9.997 14.14 11.095 76.806l5.243-5.486z"/>
        <path d="M705.16 292.54c-5.615-35.752-25.082-67.015-25.082-67.015 7.35 15.128 20.257 53.835 23.64 77.45l2.33-2.24-.888-8.195z"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="499.25" height="732.44">
    <!-- don't merge paths inheriting forbidden styles -->
    <g fill="#ffe900" fill-rule="evenodd" stroke="#1b1918">
        <g stroke-width="2.52">
            <path d="M373.27 534.98c-8.092-54.74-4.391-98.636 56.127-90.287 77.894 55.595-9.147 98.206-5.311 151.74 21.027 45.08 17.096 66.495-7.512 68.302-17.258 10.998-32.537 13.238-46.236 8.48-.246-1.867-.69-3.845-1.368-5.94l-19.752-40.751c44.709 19.982 82.483-.171 51.564-24.28zm32.16-40.207c-5.449-9.977 3.342-14.397 8.048-3.55 12.4 31.857 6.043 40.206-16.136 72.254l-1.911-2.463c11.558-13.292 20.249-27.75 21.334-39.194.899-9.481-5.973-16.736-11.335-27.048z"/>
            <path d="M407.72 580.04c40.745 49.516-3.991 92.385-40.977 82.64"/>
        </g>
    </g>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1221.3" height="1297.3" viewBox="0 0 1145 1216.2">
    <!-- allow merge on paths with equal attributes -->
    <g stroke="gray" stroke-width="1.46">
        <path d="M2236.1 787.25c6.625.191 11.52.01 11.828-2.044-8.189-9.2 8.854-46.86-11.828-48.722-17.83 3.99-6.438 26.66-11.828 48.722-.133 2.352 7.537 2.028 11.828 2.044z" transform="matrix(-.02646 -1.4538 -1.2888 .02985 1465.1 3284.4)"/>
        <path d="M2243.9 787.13c-7.561-19.76 6.33-43.05-7.817-50.642" transform="matrix(-.02646 -1.4538 -1.2888 .02985 1465.1 3284.4)"/>
        <path d="M2238.8 787.31c-4.873-19.48 2.772-37.1-2.667-50.82" transform="matrix(-.02646 -1.4538 -1.2888 .02985 1465.1 3284.4)"/>
        <path d="M2228.3 787.13c4.104-21.9-3.13-44.68 7.817-50.642" transform="matrix(-.02646 -1.4538 -1.2888 .02985 1465.1 3284.4)"/>
        <path d="M2233.4 787.31c-.692-5.383-1.098-39.17 2.667-50.82" transform="matrix(-.02646 -1.4538 -1.2888 .02985 1465.1 3284.4)"/>
    </g>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg width="100" height="100">
    <!-- don't merge where paths lose their ends and markers are rendered incorrectly -->
    <defs>
        <style>
            .a {marker-end: url(#arrowhead_end);}
        </style>
        <marker id="arrowhead_end" markerWidth="10" markerHeight="10" refX="6" refY="3">
            <path d="M 0,0 l 6,3 l -6,3" stroke="black" />
        </marker>
    </defs>
    <path d="M 10,10 h50" stroke="black" marker-end="url(#arrowhead_end)" />
    <path d="M 10,50 h50" stroke="black" marker-end="url(#arrowhead_end)" />
    <path d="M 10,60 h60" stroke="black" class="a" />
    <path d="M 10,70 h60" stroke="black" class="a"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 300">
    <!-- don't merge paths with a linearGradient fill -->
    <style>
        path.lg{fill:url(#gradient);}
    </style>
    <linearGradient id="gradient">
        <stop offset="0" stop-color="#ff0000"/>
        <stop offset="1" stop-color="#0000ff"/>
    </linearGradient>
    <path fill="url(#gradient)" d="M 0 0 H 100 V 80 H 0 z"/>
    <path fill="url(#gradient)" d="M 200 0 H 300 V 80 H 200 z"/>
    <path style="fill:url(#gradient)" d="M 0 100 h 100 v 80 H 0 z"/>
    <path style="fill:url(#gradient)" d="M 200 100 H 300 v 80 H 200 z"/>
    <path class="lg" d="M 0 200 h 100 v 80 H 0 z"/>
    <path class="lg" d="M 200 200 H 300 v 80 H 200 z"/>
</svg>"##
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -5 300 300">
    <!-- don't merge paths with a filter url -->
    <style>
        path.lg{filter:url(#blurMe);}
    </style>
    <filter id="blurMe" x=".1">
        <feGaussianBlur stdDeviation="5"/>
    </filter>
    <path filter="url(#blurMe)" fill="red" d="M 0 0 H 100 V 80 H 0 z"/>
    <path filter="url(#blurMe)" fill="red" d="M 200 0 H 300 V 80 H 200 z"/>
    <path style="filter:url(#blurMe)" fill="red" d="M 0 100 h 100 v 80 H 0 z"/>
    <path style="filter:url(#blurMe)" fill="red" d="M 200 100 H 300 v 80 H 200 z"/>
    <path class="lg" fill="red" d="M 0 200 h 100 v 80 H 0 z"/>
    <path class="lg" fill="red" d="M 200 200 H 300 v 80 H 200 z"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -5 400 400">
    <!-- don't merge paths with a clip-path -->
    <style>
        path.lg{clip-path:url(#myClip);}
    </style>
    <clipPath id="myClip" clipPathUnits="objectBoundingBox">
        <circle cx=".5" cy=".5" r=".5"/>
    </clipPath>
    <path clip-path="url(#myClip)" fill="red" d="M 0 0 H 100 V 80 H 0 z"/>
    <path clip-path="url(#myClip)" fill="red" d="M 200 0 H 300 V 80 H 200 z"/>
    <path style="clip-path:url(#myClip)" fill="red" d="M 0 100 h 100 v 80 H 0 z"/>
    <path style="clip-path:url(#myClip)" fill="red" d="M 200 100 H 300 v 80 H 200 z"/>
    <path class="lg" fill="red" d="M 0 200 h 100 v 80 H 0 z"/>
    <path class="lg" fill="red" d="M 200 200 H 300 v 80 H 200 z"/>
    <path style="clip-path:circle(25%)" fill="red" d="M 0 300 h 100 v 80 H 0 z"/>
    <path style="clip-path:circle(25%)" fill="red" d="M 200 300 H 300 v 80 H 200 z"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-5 -5 400 400">
    <!-- don't merge paths with a mask -->
    <style>
        path.lg{mask:url(#mask);}
    </style>
    <mask id="mask" maskContentUnits="objectBoundingBox">
        <rect fill="white" x="0" y="0" width="100%" height="100%"/>
        <circle fill="black" cx=".5" cy=".5" r=".5"/>
    </mask>
    <path mask="url(#mask)" fill="red" d="M 0 0 H 100 V 80 H 0 z"/>
    <path mask="url(#mask)" fill="red" d="M 200 0 H 300 V 80 H 200 z"/>
    <path style="mask:url(#mask)" fill="red" d="M 0 100 h 100 v 80 H 0 z"/>
    <path style="mask:url(#mask)" fill="red" d="M 200 100 H 300 v 80 H 200 z"/>
    <path class="lg" fill="red" d="M 0 200 h 100 v 80 H 0 z"/>
    <path class="lg" fill="red" d="M 200 200 H 300 v 80 H 200 z"/>
    <path style="mask-image: linear-gradient(to left top,black, transparent)" fill="red" d="M 0 300 h 100 v 80 H 0 z"/>
    <path style="mask-image: linear-gradient(to left top,black, transparent)" fill="red" d="M 200 300 H 300 v 80 H 200 z"/>
</svg>"#
        ),
    )?);

    insta::assert_snapshot!(test_config(
        r#"{ "mergePaths": {} }"#,
        Some(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 122.764 105.935">
    <path d="M43.119 39.565Zm-.797 3.961c.077.167.257.083.309.177Z"/>
    <path d="m42.38 43.684-.06.019Z"/>
</svg>"#
        ),
    )?);

    Ok(())
}
