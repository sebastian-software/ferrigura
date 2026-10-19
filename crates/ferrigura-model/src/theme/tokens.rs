//! The theme variables as typed tokens (`docs-port/typed-views.md`, step 1).
//!
//! The variables of a theme are a map, the `themeVariables` of the configuration: colours, sizes,
//! flags, lists, and a few groups of options for particular diagrams. The renderers read them by
//! name. [`ThemeTokens`] gives each name a field, so that a renderer which names a variable the
//! themes do not produce does not compile. A token holds the value as the map held it, and its four
//! readings, [`Token::text`], [`Token::texts`], [`Token::truthy`] and [`Token::px`], give what the
//! helpers of `ferrigura_render::variables` give for the same name on the map.
//!
//! The fields are the union of the variables the two shipped redux themes produce
//! (`packages/mermaid/src/themes/theme-redux-color.js` and `theme-redux-dark-color.js`) and the names
//! that the renderers read, including the names built at run time. The indexed names (`pie1` to
//! `pie12`, `cScale0` to `cScale11`, `fillType0` to `fillType7`, and the others the themes number)
//! are [`Family`]s, read by index. The groups of the diagram types (`xyChart`, `wardley`, `packet`,
//! `treeView`, `radar`, `cynefin`) are structs of their own. A name that only a user's overrides
//! set, and that no renderer reads, is not a token: the map keeps it, and nothing reads it.

use std::collections::BTreeMap;

use crate::value::{Value, number_text};

/// One theme variable, as the map holds it: the value, or nothing when the map holds no value under
/// its name. The readings say what a value that is not text or a number gives.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Token(Option<Value>);

/// The token of a name that the map does not hold.
static ABSENT: Token = Token(None);

impl Token {
    /// The token that holds `value`, or the absent token for `None`.
    #[must_use]
    pub fn new(value: Option<&Value>) -> Self {
        Self(value.cloned())
    }

    /// The value the map held under the name, or `None` when it held none.
    #[must_use]
    pub fn value(&self) -> Option<&Value> {
        self.0.as_ref()
    }

    /// Whether the map held no value under the name.
    #[must_use]
    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }

    /// The variable as text: a string as it is, a number as the SVG writers write it, and `None` for
    /// any other value or for an absent variable.
    #[must_use]
    pub fn text(&self) -> Option<String> {
        match self.0.as_ref()? {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number_text(*number)),
            _ => None,
        }
    }

    /// The items of a list variable as text, in order. Items that are neither text nor numbers are
    /// skipped; a variable that is not a list gives no items.
    #[must_use]
    pub fn texts(&self) -> Vec<String> {
        match self.0.as_ref() {
            Some(Value::Array(items)) => items
                .iter()
                .filter_map(|item| match item {
                    Value::String(text) => Some(text.clone()),
                    Value::Number(number) => Some(number_text(*number)),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the variable is truthy as JavaScript reads it: `true`, a non-empty string, a number
    /// that is neither zero nor `NaN`, and any list or object. An absent variable is not truthy.
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self.0.as_ref() {
            Some(Value::Bool(flag)) => *flag,
            Some(Value::String(text)) => !text.is_empty(),
            Some(Value::Number(number)) => *number != 0.0 && !number.is_nan(),
            Some(Value::Array(_) | Value::Object(_)) => true,
            Some(Value::Null) | None => false,
        }
    }

    /// The leading pixel count of a font size or a stroke width, as `parseFontSize` reads it: the
    /// digits that lead the text (`25px` is `25`), or `None` when none do.
    #[must_use]
    pub fn px(&self) -> Option<f64> {
        leading_integer(&self.text()?)
    }
}

/// `parseInt(text, 10)` for the leading digits of a size such as `25px`: leading white space is
/// skipped, a sign is kept, and the result is `None` when no digit leads or the number is not finite.
#[must_use]
pub fn leading_integer(text: &str) -> Option<f64> {
    let trimmed = text.trim_start();
    let digits: String = trimmed
        .char_indices()
        .take_while(|(index, c)| c.is_ascii_digit() || (*index == 0 && (*c == '-' || *c == '+')))
        .map(|(_, c)| c)
        .collect();
    digits.parse::<f64>().ok().filter(|value| value.is_finite())
}

/// The tokens of an indexed family: the names `prefix` followed by an index, from `first` on, such
/// as `pie1` to `pie12` or `cScale0` to `cScale11`. The members the themes hold sit in fixed slots,
/// which the renderers index by number. A member that a user's overrides name outside the slots is
/// kept as well, so that [`Family::get`] reads every index as the map does.
#[derive(Debug, Clone, PartialEq)]
pub struct Family<const N: usize> {
    /// The text before the index.
    prefix: &'static str,
    /// The index of the first slot.
    first: usize,
    /// The members in the slots, in index order.
    slots: [Token; N],
    /// The members outside the slots that the map holds, by index.
    others: BTreeMap<usize, Token>,
}

impl<const N: usize> Default for Family<N> {
    fn default() -> Self {
        Self {
            prefix: "",
            first: 0,
            slots: std::array::from_fn(|_| Token::default()),
            others: BTreeMap::new(),
        }
    }
}

impl<const N: usize> Family<N> {
    /// The family of a map: the members `prefix` followed by the indices `first` to `first + N - 1`,
    /// and any other index that the map holds under the prefix.
    #[must_use]
    pub fn new(map: &Value, prefix: &'static str, first: usize) -> Self {
        let slots =
            std::array::from_fn(|slot| Token::new(map.get(&format!("{prefix}{}", first + slot))));
        let others = match map.as_object() {
            Some(entries) => entries
                .iter()
                .filter_map(|(key, value)| {
                    let index = index_of(key, prefix)?;
                    let in_slots = index.checked_sub(first).is_some_and(|slot| slot < N);
                    (!in_slots).then(|| (index, Token::new(Some(value))))
                })
                .collect(),
            None => BTreeMap::new(),
        };
        Self {
            prefix,
            first,
            slots,
            others,
        }
    }

    /// The token of the member with the index: the name `prefix` followed by `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> &Token {
        match index.checked_sub(self.first) {
            Some(slot) if slot < N => &self.slots[slot],
            _ => self.others.get(&index).unwrap_or(&ABSENT),
        }
    }

    /// The name of the member with the index, which is the name the map holds it under.
    #[must_use]
    pub fn name(&self, index: usize) -> String {
        format!("{}{index}", self.prefix)
    }

    /// The members in the slots with their names, in index order.
    #[must_use]
    pub fn entries(&self) -> Vec<(String, &Token)> {
        self.slots
            .iter()
            .enumerate()
            .map(|(slot, token)| (self.name(self.first + slot), token))
            .collect()
    }
}

/// The index that `key` gives after `prefix`, when the key is the prefix followed by the index as
/// `format!` writes it: decimal digits with no sign and no leading zero.
fn index_of(key: &str, prefix: &str) -> Option<usize> {
    let digits = key.strip_prefix(prefix)?;
    let canonical = !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits == "0" || !digits.starts_with('0'));
    if canonical { digits.parse().ok() } else { None }
}

/// Declares a struct of tokens from one list of `field = "jsName": kind`: a `token` is a [`Token`],
/// a `family(N, first)` is a [`Family`] of `N` members from index `first`, and a `group(Type)` is an
/// `Option` of the struct `Type` that the map holds as an object under the name.
macro_rules! token_struct {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            $( $field:ident = $js:literal : $kind:ident $( ( $($arg:tt)* ) )? ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct $name {
            $(
                #[doc = concat!("The `", $js, "` variable: for a family, its members; for a group, the group.")]
                pub $field: token_struct!(@field $kind $( ( $($arg)* ) )?),
            )*
        }

        impl $name {
            /// The names of the variables of this struct, as the map holds them. A family is named by
            /// its prefix; its members are the prefix and an index, see [`Family::name`].
            pub const NAMES: &'static [&'static str] = &[ $( $js ),* ];

            /// The tokens of a map of theme variables, or of a group of them: each token holds the
            /// value that the map holds under its name, and is absent when the map holds none. A group
            /// is present when the map holds an object under its name.
            #[must_use]
            pub fn from_value(map: &Value) -> Self {
                Self {
                    $( $field: token_struct!(@new map, $js, $kind $( ( $($arg)* ) )?), )*
                }
            }

            /// The tokens of the struct with their names: the variables, and the members of each family
            /// in its slots. The groups are not included; each group has its own `entries`.
            #[must_use]
            pub fn entries(&self) -> Vec<(String, &Token)> {
                let mut entries = Vec::new();
                $( token_struct!(@entry entries, self.$field, $js, $kind $( ( $($arg)* ) )?); )*
                entries
            }
        }
    };
    (@field token) => { Token };
    (@field family ( $n:literal, $first:literal )) => { Family<$n> };
    (@field group ( $group:ident )) => { Option<$group> };
    (@new $map:ident, $js:literal, token) => { Token::new($map.get($js)) };
    (@new $map:ident, $js:literal, family ( $n:literal, $first:literal )) => {
        Family::new($map, $js, $first)
    };
    (@new $map:ident, $js:literal, group ( $group:ident )) => {
        $map.get($js).filter(|value| value.is_object()).map($group::from_value)
    };
    (@entry $entries:ident, $field:expr, $js:literal, token) => {
        $entries.push(($js.to_owned(), &$field));
    };
    (@entry $entries:ident, $field:expr, $js:literal, family ( $n:literal, $first:literal )) => {
        $entries.extend($field.entries());
    };
    (@entry $entries:ident, $field:expr, $js:literal, group ( $group:ident )) => {};
}

token_struct! {
    /// The `xyChart` group of the theme variables: the colours of the XY chart, which the redux
    /// themes compute as an object.
    pub struct XyChartTokens {
        background_color = "backgroundColor": token,
        data_label_color = "dataLabelColor": token,
        legend_text_color = "legendTextColor": token,
        plot_color_palette = "plotColorPalette": token,
        title_color = "titleColor": token,
        x_axis_label_color = "xAxisLabelColor": token,
        x_axis_line_color = "xAxisLineColor": token,
        x_axis_tick_color = "xAxisTickColor": token,
        x_axis_title_color = "xAxisTitleColor": token,
        y_axis_label_color = "yAxisLabelColor": token,
        y_axis_line_color = "yAxisLineColor": token,
        y_axis_tick_color = "yAxisTickColor": token,
        y_axis_title_color = "yAxisTitleColor": token,
    }
}

token_struct! {
    /// The `wardley` group of the theme variables: the colours of the wardley map, which a user's
    /// overrides may set (`wardley/theme.rs`). The redux themes give no group. The reference reads
    /// `annotationStroke`, `annotationTextColor` and `annotationFill` too, but nothing it draws takes
    /// them, so they are not tokens.
    pub struct WardleyTokens {
        background_color = "backgroundColor": token,
        axis_color = "axisColor": token,
        axis_text_color = "axisTextColor": token,
        grid_color = "gridColor": token,
        component_fill = "componentFill": token,
        component_stroke = "componentStroke": token,
        component_label_color = "componentLabelColor": token,
        link_stroke = "linkStroke": token,
        evolution_stroke = "evolutionStroke": token,
    }
}

token_struct! {
    /// The `packet` group of the theme variables: the options of the packet diagram's styles
    /// (`packet/style.rs`, the defaults of `styles.ts`), which a user's overrides may set.
    pub struct PacketTokens {
        byte_font_size = "byteFontSize": token,
        start_byte_color = "startByteColor": token,
        end_byte_color = "endByteColor": token,
        label_color = "labelColor": token,
        label_font_size = "labelFontSize": token,
        title_color = "titleColor": token,
        title_font_size = "titleFontSize": token,
        block_stroke_color = "blockStrokeColor": token,
        block_stroke_width = "blockStrokeWidth": token,
        block_fill_color = "blockFillColor": token,
    }
}

token_struct! {
    /// The `treeView` group of the theme variables: the styles of the tree view (`styles.ts`, the
    /// defaults of `defaultTreeViewDiagramStyles`), which a user's overrides may set. No shipped
    /// theme sets the group.
    pub struct TreeViewTokens {
        label_font_size = "labelFontSize": token,
        label_color = "labelColor": token,
        line_color = "lineColor": token,
        icon_color = "iconColor": token,
        description_color = "descriptionColor": token,
        highlight_bg = "highlightBg": token,
        highlight_stroke = "highlightStroke": token,
    }
}

token_struct! {
    /// The `radar` group of the theme variables: the options of the radar diagram's styles
    /// (`radar/style.rs`, the merge of `styles.ts` 35-38 over the default theme's `radar` section,
    /// `theme-default.js` 338-350). The redux themes give no group, so the defaults apply; a user's
    /// `themeVariables.radar` sets any of these. `legendBoxSize` is not in the list: no reference code
    /// reads it.
    pub struct RadarTokens {
        axis_color = "axisColor": token,
        axis_stroke_width = "axisStrokeWidth": token,
        axis_label_font_size = "axisLabelFontSize": token,
        curve_opacity = "curveOpacity": token,
        curve_stroke_width = "curveStrokeWidth": token,
        graticule_color = "graticuleColor": token,
        graticule_opacity = "graticuleOpacity": token,
        graticule_stroke_width = "graticuleStrokeWidth": token,
        legend_font_size = "legendFontSize": token,
    }
}

token_struct! {
    /// The `cynefin` group of the theme variables: the colours and sizes of the Cynefin diagram. The
    /// default theme computes the group (`theme-default.js`, the `cynefin` block), and a theme or a
    /// user's override sets single keys of it; the renderer takes the defaults for the keys it does not
    /// find (`cynefin/style.rs`).
    pub struct CynefinTokens {
        domain_font_size = "domainFontSize": token,
        item_font_size = "itemFontSize": token,
        boundary_color = "boundaryColor": token,
        boundary_width = "boundaryWidth": token,
        cliff_color = "cliffColor": token,
        cliff_width = "cliffWidth": token,
        arrow_color = "arrowColor": token,
        arrow_width = "arrowWidth": token,
        complex_bg = "complexBg": token,
        complicated_bg = "complicatedBg": token,
        chaotic_bg = "chaoticBg": token,
        clear_bg = "clearBg": token,
        confusion_bg = "confusionBg": token,
        text_color = "textColor": token,
        label_color = "labelColor": token,
    }
}

token_struct! {
    /// The theme variables as typed tokens: one field per variable of the redux themes and of the
    /// renderers. Built once per resolved theme with [`ThemeTokens::from_value`].
    pub struct ThemeTokens {
        background = "background": token,
        primary_color = "primaryColor": token,
        main_bkg = "mainBkg": token,
        note_bkg_color = "noteBkgColor": token,
        note_text_color = "noteTextColor": token,
        theme_color_limit = "THEME_COLOR_LIMIT": token,
        radius = "radius": token,
        stroke_width = "strokeWidth": token,
        primary_border_color = "primaryBorderColor": token,
        dark_mode = "darkMode": token,
        font_family = "fontFamily": token,
        font_size = "fontSize": token,
        node_border = "nodeBorder": token,
        state_border = "stateBorder": token,
        use_gradient = "useGradient": token,
        gradient_start = "gradientStart": token,
        gradient_stop = "gradientStop": token,
        second_bkg = "secondBkg": token,
        drop_shadow = "dropShadow": token,
        node_shadow = "nodeShadow": token,
        main_contrast_color = "mainContrastColor": token,
        tertiary_color = "tertiaryColor": token,
        dark_text_color = "darkTextColor": token,
        cluster_bkg = "clusterBkg": token,
        cluster_border = "clusterBorder": token,
        arch_edge_color = "archEdgeColor": token,
        arch_edge_arrow_color = "archEdgeArrowColor": token,
        arch_edge_width = "archEdgeWidth": token,
        arch_group_border_color = "archGroupBorderColor": token,
        label_background = "labelBackground": token,
        arch_group_border_width = "archGroupBorderWidth": token,
        actor_border = "actorBorder": token,
        note_border_color = "noteBorderColor": token,
        note_font_weight = "noteFontWeight": token,
        border_color_array = "borderColorArray": token,
        bkg_color_array = "bkgColorArray": token,
        border = "border": family(2, 1),
        usecase_actor_border = "usecaseActorBorder": token,
        usecase_actor_bkg = "usecaseActorBkg": token,
        usecase_border = "usecaseBorder": token,
        usecase_bkg = "usecaseBkg": token,
        usecase_boundary_border = "usecaseBoundaryBorder": token,
        usecase_boundary_bkg = "usecaseBoundaryBkg": token,
        usecase_include_line = "usecaseIncludeLine": token,
        usecase_extend_line = "usecaseExtendLine": token,
        filter_color = "filterColor": token,
        primary_text_color = "primaryTextColor": token,
        secondary_color = "secondaryColor": token,
        secondary_border_color = "secondaryBorderColor": token,
        tertiary_border_color = "tertiaryBorderColor": token,
        secondary_text_color = "secondaryTextColor": token,
        tertiary_text_color = "tertiaryTextColor": token,
        line_color = "lineColor": token,
        arrowhead_color = "arrowheadColor": token,
        text_color = "textColor": token,
        node_bkg = "nodeBkg": token,
        default_link_color = "defaultLinkColor": token,
        title_color = "titleColor": token,
        edge_label_background = "edgeLabelBackground": token,
        node_text_color = "nodeTextColor": token,
        flow_container_stroke = "flowContainerStroke": token,
        actor_bkg = "actorBkg": token,
        actor_text_color = "actorTextColor": token,
        actor_line_color = "actorLineColor": token,
        label_box_bkg_color = "labelBoxBkgColor": token,
        signal_color = "signalColor": token,
        signal_text_color = "signalTextColor": token,
        label_box_border_color = "labelBoxBorderColor": token,
        label_text_color = "labelTextColor": token,
        loop_text_color = "loopTextColor": token,
        activation_border_color = "activationBorderColor": token,
        activation_bkg_color = "activationBkgColor": token,
        sequence_number_color = "sequenceNumberColor": token,
        rect_bkg_color = "rectBkgColor": token,
        exclude_bkg_color = "excludeBkgColor": token,
        task_border_color = "taskBorderColor": token,
        task_bkg_color = "taskBkgColor": token,
        active_task_border_color = "activeTaskBorderColor": token,
        active_task_bkg_color = "activeTaskBkgColor": token,
        root_label_color = "rootLabelColor": token,
        grid_color = "gridColor": token,
        done_task_bkg_color = "doneTaskBkgColor": token,
        done_task_border_color = "doneTaskBorderColor": token,
        crit_border_color = "critBorderColor": token,
        crit_bkg_color = "critBkgColor": token,
        today_line_color = "todayLineColor": token,
        task_text_color = "taskTextColor": token,
        vert_line_color = "vertLineColor": token,
        task_text_outside_color = "taskTextOutsideColor": token,
        task_text_light_color = "taskTextLightColor": token,
        task_text_dark_color = "taskTextDarkColor": token,
        task_text_clickable_color = "taskTextClickableColor": token,
        person_border = "personBorder": token,
        person_bkg = "personBkg": token,
        transition_color = "transitionColor": token,
        transition_label_color = "transitionLabelColor": token,
        state_label_color = "stateLabelColor": token,
        state_bkg = "stateBkg": token,
        state_edge_label_background = "stateEdgeLabelBackground": token,
        label_background_color = "labelBackgroundColor": token,
        composite_background = "compositeBackground": token,
        alt_background = "altBackground": token,
        composite_title_background = "compositeTitleBackground": token,
        composite_border = "compositeBorder": token,
        inner_end_background = "innerEndBackground": token,
        error_bkg_color = "errorBkgColor": token,
        error_text_color = "errorTextColor": token,
        special_state_color = "specialStateColor": token,
        c_scale = "cScale": family(12, 0),
        section_bkg_color = "sectionBkgColor": token,
        alt_section_bkg_color = "altSectionBkgColor": token,
        section_bkg_color2 = "sectionBkgColor2": token,
        c_scale_inv = "cScaleInv": family(12, 0),
        c_scale_peer = "cScalePeer": family(12, 0),
        scale_label_color = "scaleLabelColor": token,
        c_scale_label = "cScaleLabel": family(12, 0),
        surface = "surface": family(5, 0),
        surface_peer = "surfacePeer": family(5, 0),
        class_text = "classText": token,
        fill_type = "fillType": family(8, 0),
        pie = "pie": family(12, 1),
        pie_title_text_size = "pieTitleTextSize": token,
        pie_title_text_color = "pieTitleTextColor": token,
        pie_section_text_size = "pieSectionTextSize": token,
        pie_section_text_color = "pieSectionTextColor": token,
        pie_legend_text_size = "pieLegendTextSize": token,
        pie_legend_text_color = "pieLegendTextColor": token,
        pie_stroke_color = "pieStrokeColor": token,
        pie_stroke_width = "pieStrokeWidth": token,
        pie_outer_stroke_width = "pieOuterStrokeWidth": token,
        pie_outer_stroke_color = "pieOuterStrokeColor": token,
        pie_opacity = "pieOpacity": token,
        venn = "venn": family(8, 1),
        venn_title_text_color = "vennTitleTextColor": token,
        venn_set_text_color = "vennSetTextColor": token,
        quadrant1_fill = "quadrant1Fill": token,
        quadrant2_fill = "quadrant2Fill": token,
        quadrant3_fill = "quadrant3Fill": token,
        quadrant4_fill = "quadrant4Fill": token,
        quadrant1_text_fill = "quadrant1TextFill": token,
        quadrant2_text_fill = "quadrant2TextFill": token,
        quadrant3_text_fill = "quadrant3TextFill": token,
        quadrant4_text_fill = "quadrant4TextFill": token,
        quadrant_point_fill = "quadrantPointFill": token,
        quadrant_point_text_fill = "quadrantPointTextFill": token,
        quadrant_x_axis_text_fill = "quadrantXAxisTextFill": token,
        quadrant_y_axis_text_fill = "quadrantYAxisTextFill": token,
        quadrant_internal_border_stroke_fill = "quadrantInternalBorderStrokeFill": token,
        quadrant_external_border_stroke_fill = "quadrantExternalBorderStrokeFill": token,
        quadrant_title_fill = "quadrantTitleFill": token,
        xy_chart = "xyChart": group(XyChartTokens),
        requirement_background = "requirementBackground": token,
        requirement_border_color = "requirementBorderColor": token,
        requirement_border_size = "requirementBorderSize": token,
        requirement_text_color = "requirementTextColor": token,
        relation_color = "relationColor": token,
        relation_label_background = "relationLabelBackground": token,
        relation_label_color = "relationLabelColor": token,
        requirement_edge_label_background = "requirementEdgeLabelBackground": token,
        git = "git": family(8, 0),
        branch_label_color = "branchLabelColor": token,
        tag_label_color = "tagLabelColor": token,
        tag_label_background = "tagLabelBackground": token,
        tag_label_border = "tagLabelBorder": token,
        tag_label_font_size = "tagLabelFontSize": token,
        commit_label_color = "commitLabelColor": token,
        commit_label_background = "commitLabelBackground": token,
        commit_line_color = "commitLineColor": token,
        commit_label_font_size = "commitLabelFontSize": token,
        font_weight = "fontWeight": token,
        er_edge_label_background = "erEdgeLabelBackground": token,
        em_arrowhead = "emArrowhead": token,
        em_command_fill = "emCommandFill": token,
        em_command_stroke = "emCommandStroke": token,
        em_event_fill = "emEventFill": token,
        em_event_stroke = "emEventStroke": token,
        em_processor_fill = "emProcessorFill": token,
        em_processor_stroke = "emProcessorStroke": token,
        em_read_model_fill = "emReadModelFill": token,
        em_read_model_stroke = "emReadModelStroke": token,
        em_relation_stroke = "emRelationStroke": token,
        em_swimlane_background_odd = "emSwimlaneBackgroundOdd": token,
        em_swimlane_background_stroke = "emSwimlaneBackgroundStroke": token,
        em_ui_fill = "emUiFill": token,
        em_ui_stroke = "emUiStroke": token,
        git_branch_label = "gitBranchLabel": family(8, 0),
        git_inv = "gitInv": family(8, 0),
        packet = "packet": group(PacketTokens),
        row_even = "rowEven": token,
        row_odd = "rowOdd": token,

        // The tree view's group (P3, `treeView`): a block of its own, at the end of the list.
        tree_view = "treeView": group(TreeViewTokens),

        // The radar diagram's block: its group of the theme variables, at the end of the list.
        radar = "radar": group(RadarTokens),

        // The wardley block: the group of the wardley map (`wardley/theme.rs`).
        wardley = "wardley": group(WardleyTokens),

        // cynefin (P3 renderer): the group of the Cynefin diagram's colours and sizes.
        cynefin = "cynefin": group(CynefinTokens),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;

    #[test]
    fn every_name_the_redux_themes_produce_is_a_token() {
        for theme in [Theme::ReduxColor, Theme::ReduxDarkColor] {
            let variables = theme.variables(None);
            let tokens = ThemeTokens::from_value(&variables);
            let names: Vec<String> = tokens.entries().into_iter().map(|(name, _)| name).collect();
            for name in variables
                .as_object()
                .expect("the variables are an object")
                .keys()
            {
                if name == "xyChart" {
                    assert!(tokens.xy_chart.is_some(), "the group {name} is not a token");
                    let xy = variables
                        .get(name)
                        .and_then(Value::as_object)
                        .expect("an object");
                    for key in xy.keys() {
                        assert!(
                            XyChartTokens::NAMES.contains(&key.as_str()),
                            "xyChart.{key} of {theme:?} is not a token"
                        );
                    }
                } else {
                    assert!(names.contains(name), "{name} of {theme:?} is not a token");
                }
            }
        }
    }

    #[test]
    fn a_name_is_listed_once() {
        let mut names = ThemeTokens::NAMES.to_vec();
        names.sort_unstable();
        let listed = names.len();
        names.dedup();
        assert_eq!(listed, names.len());
    }

    #[test]
    fn the_readings_of_a_token_follow_the_helpers() {
        let map = Value::from(serde_json::json!({
            "fontSize": "25px",
            "strokeWidth": 2,
            "useGradient": false,
            "borderColorArray": ["#f00", 3, null, true],
            "primaryColor": [1],
            "darkMode": "",
        }));
        let tokens = ThemeTokens::from_value(&map);
        assert_eq!(tokens.font_size.text().as_deref(), Some("25px"));
        assert_eq!(tokens.font_size.px(), Some(25.0));
        assert_eq!(tokens.stroke_width.text().as_deref(), Some("2"));
        assert_eq!(tokens.stroke_width.px(), Some(2.0));
        assert!(!tokens.use_gradient.truthy());
        assert_eq!(tokens.border_color_array.texts(), ["#f00", "3"]);
        assert_eq!(tokens.primary_color.text(), None);
        assert!(tokens.primary_color.truthy());
        assert!(!tokens.dark_mode.truthy());
        assert!(tokens.line_color.is_absent());
        assert!(!tokens.line_color.truthy());
        assert!(tokens.line_color.texts().is_empty());
        assert_eq!(tokens.line_color.px(), None);
    }

    #[test]
    fn a_family_reads_its_slots_and_the_members_outside_them() {
        let map = Value::from(serde_json::json!({
            "pie1": "#111", "pie12": "#222", "pie13": "#333", "pie01": "#444", "pie": "#555",
        }));
        let tokens = ThemeTokens::from_value(&map);
        assert_eq!(tokens.pie.get(1).text().as_deref(), Some("#111"));
        assert_eq!(tokens.pie.get(12).text().as_deref(), Some("#222"));
        assert_eq!(tokens.pie.get(13).text().as_deref(), Some("#333"));
        assert!(tokens.pie.get(0).is_absent());
        assert!(tokens.pie.get(2).is_absent());
        assert_eq!(tokens.pie.name(13), "pie13");
    }

    #[test]
    fn a_group_is_present_for_an_object_only() {
        let object = ThemeTokens::from_value(&Value::from(serde_json::json!({
            "xyChart": {"titleColor": "#123"},
        })));
        assert_eq!(
            object.xy_chart.as_ref().map(|xy| xy.title_color.text()),
            Some(Some("#123".to_owned()))
        );
        let text = ThemeTokens::from_value(&Value::from(serde_json::json!({"xyChart": "#123"})));
        assert!(text.xy_chart.is_none());
    }
}
