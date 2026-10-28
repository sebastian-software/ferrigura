//! The chart builder: `chartBuilder/` of the reference. [`build`] lays the chart out and returns the
//! elements to draw, in the order the reference's orchestrator lists its components: the title, the
//! plots, the legend, the x axis and the y axis (`orchestrator.ts` 209-217).
//!
//! The layout is the reference's arithmetic, step by step, with the same floors and the same order
//! of the calls, because the axes and the plots share state that each call changes.

pub mod axis;
pub mod interfaces;
pub mod legend;
pub mod plot;
pub mod text;
pub mod title;

use axis::{Axis, AxisPosition};
use interfaces::{ChartConfig, ChartData, Dimension, Drawable, PlotData, ThemeConfig};
use legend::ChartLegend;
use plot::Plot;
use title::ChartTitle;

use crate::RenderError;

/// `XYChartBuilder.build`: the drawable elements of the chart.
///
/// # Errors
///
/// Returns an error when the chart has no plot (the reference throws `No Plot to render`), or when
/// the bundled font cannot be read.
pub fn build(
    config: &ChartConfig,
    data: &ChartData,
    theme: &ThemeConfig,
) -> Result<Vec<Drawable>, RenderError> {
    if data.plots.is_empty() {
        return Err(RenderError::Layout(
            "No Plot to render, please provide a plot with some data".to_owned(),
        ));
    }
    let mut orchestrator = Orchestrator::new(config, data, theme);
    orchestrator.calculate_space()?;
    orchestrator.drawables()
}

/// The components of the chart and the layout that places them.
struct Orchestrator<'a> {
    config: &'a ChartConfig,
    data: &'a ChartData,
    title: ChartTitle<'a>,
    plot: Plot<'a>,
    legend: ChartLegend<'a>,
    x_axis: Axis,
    y_axis: Axis,
}

impl<'a> Orchestrator<'a> {
    fn new(config: &'a ChartConfig, data: &'a ChartData, theme: &'a ThemeConfig) -> Self {
        Self {
            config,
            data,
            title: ChartTitle::new(config, data, theme),
            plot: Plot::new(config, data),
            legend: ChartLegend::new(config, data, theme),
            x_axis: Axis::new(
                &data.x_axis,
                &config.x_axis,
                &theme.x_axis,
                config.screen_scale,
            ),
            y_axis: Axis::new(
                &data.y_axis,
                &config.y_axis,
                &theme.y_axis,
                config.screen_scale,
            ),
        }
    }

    fn has_bar(&self) -> bool {
        self.data
            .plots
            .iter()
            .any(|plot| matches!(plot, PlotData::Bar { .. }))
    }

    /// `calculateVerticalSpace`: the x axis at the bottom, the y axis at the left.
    fn calculate_vertical_space(&mut self) -> Result<(), RenderError> {
        let config = self.config;
        let mut available_width = config.width;
        let mut available_height = config.height;
        let mut chart_width =
            ((available_width * config.plot_reserved_space_percent) / 100.0).floor();
        let mut chart_height =
            ((available_height * config.plot_reserved_space_percent) / 100.0).floor();
        let used = self.plot.calculate_space(Dimension {
            width: chart_width,
            height: chart_height,
        });
        available_width -= used.width;
        available_height -= used.height;

        let used = self.title.calculate_space(Dimension {
            width: config.width,
            height: available_height,
        })?;
        let plot_y = used.height;
        available_height -= used.height;

        self.x_axis.set_axis_position(AxisPosition::Bottom);
        let used = self.x_axis.calculate_space(Dimension {
            width: available_width,
            height: available_height,
        })?;
        available_height -= used.height;

        self.y_axis.set_axis_position(AxisPosition::Left);
        let used = self.y_axis.calculate_space(Dimension {
            width: available_width,
            height: available_height,
        })?;
        let plot_x = used.width;
        available_width -= used.width;

        let legend_space = self.legend.calculate_space(Dimension {
            width: available_width,
            height: chart_height,
        })?;
        available_width -= legend_space.width;
        if available_width > 0.0 {
            chart_width += available_width;
        }
        if available_height > 0.0 {
            chart_height += available_height;
        }
        self.plot.calculate_space(Dimension {
            width: chart_width,
            height: chart_height,
        });

        self.place(
            plot_x,
            plot_y,
            chart_width,
            chart_height,
            legend_space,
            |orchestrator| {
                orchestrator
                    .x_axis
                    .set_range([plot_x, plot_x + chart_width]);
                orchestrator
                    .x_axis
                    .set_bounding_box_xy(plot_x, plot_y + chart_height);
                orchestrator
                    .y_axis
                    .set_range([plot_y, plot_y + chart_height]);
                orchestrator.y_axis.set_bounding_box_xy(0.0, plot_y);
            },
        );
        Ok(())
    }

    /// `calculateHorizontalSpace`: the x axis on the left, the y axis at the top.
    fn calculate_horizontal_space(&mut self) -> Result<(), RenderError> {
        let config = self.config;
        let mut available_width = config.width;
        let mut available_height = config.height;
        let mut chart_width =
            ((available_width * config.plot_reserved_space_percent) / 100.0).floor();
        let mut chart_height =
            ((available_height * config.plot_reserved_space_percent) / 100.0).floor();
        let used = self.plot.calculate_space(Dimension {
            width: chart_width,
            height: chart_height,
        });
        available_width -= used.width;
        available_height -= used.height;

        let used = self.title.calculate_space(Dimension {
            width: config.width,
            height: available_height,
        })?;
        let title_y_end = used.height;
        available_height -= used.height;

        self.x_axis.set_axis_position(AxisPosition::Left);
        let used = self.x_axis.calculate_space(Dimension {
            width: available_width,
            height: available_height,
        })?;
        available_width -= used.width;
        let plot_x = used.width;

        self.y_axis.set_axis_position(AxisPosition::Top);
        let used = self.y_axis.calculate_space(Dimension {
            width: available_width,
            height: available_height,
        })?;
        available_height -= used.height;
        let plot_y = title_y_end + used.height;

        let legend_space = self.legend.calculate_space(Dimension {
            width: available_width,
            height: chart_height,
        })?;
        available_width -= legend_space.width;
        if available_width > 0.0 {
            chart_width += available_width;
        }
        if available_height > 0.0 {
            chart_height += available_height;
        }
        self.plot.calculate_space(Dimension {
            width: chart_width,
            height: chart_height,
        });

        self.place(
            plot_x,
            plot_y,
            chart_width,
            chart_height,
            legend_space,
            |orchestrator| {
                orchestrator
                    .y_axis
                    .set_range([plot_x, plot_x + chart_width]);
                orchestrator.y_axis.set_bounding_box_xy(plot_x, title_y_end);
                orchestrator
                    .x_axis
                    .set_range([plot_y, plot_y + chart_height]);
                orchestrator.x_axis.set_bounding_box_xy(0.0, plot_y);
            },
        );
        Ok(())
    }

    /// The placements that both orientations share: the plot, the title, the legend, and the axes
    /// through `axes`, then the outer padding of the bars.
    fn place(
        &mut self,
        plot_x: f64,
        plot_y: f64,
        chart_width: f64,
        chart_height: f64,
        legend_space: Dimension,
        axes: impl FnOnce(&mut Self),
    ) {
        self.plot.set_bounding_box_xy(plot_x, plot_y);
        self.title.align_to(plot_x, plot_x + chart_width);
        self.legend.set_bounding_box_xy(
            plot_x + chart_width,
            plot_y + ((chart_height - legend_space.height) / 2.0).max(0.0),
        );
        axes(self);
        if self.has_bar() {
            self.x_axis.recalculate_outer_padding_to_draw_bar();
        }
    }

    fn calculate_space(&mut self) -> Result<(), RenderError> {
        if self.config.horizontal {
            self.calculate_horizontal_space()
        } else {
            self.calculate_vertical_space()
        }
    }

    /// `getDrawableElement()`: the components' elements, in the order of the component store.
    fn drawables(&mut self) -> Result<Vec<Drawable>, RenderError> {
        let mut out = Vec::new();
        out.extend(self.title.drawables());
        out.extend(self.plot.drawables(&self.x_axis, &self.y_axis));
        out.extend(self.legend.drawables());
        out.extend(self.x_axis.get_drawable_elements()?);
        out.extend(self.y_axis.get_drawable_elements()?);
        Ok(out)
    }
}
