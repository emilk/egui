# Changelog for egui_plot
All notable changes to the `egui_plot` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.24.0 - 2023-11-23
* Add `emath::Vec2b`, replacing `egui_plot::AxisBools` [#3543](https://github.com/emilk/egui/pull/3543)
* Add `auto_bounds/set_auto_bounds` to `PlotUi` [#3587](https://github.com/emilk/egui/pull/3587) [#3586](https://github.com/emilk/egui/pull/3586) (thanks [@abey79](https://github.com/abey79)!)
* Update MSRV to Rust 1.72 [#3595](https://github.com/emilk/egui/pull/3595)


## 0.23.0 - 2023-09-27 - Initial release, after being forked out from `egui`
* Draw axis labels and ticks outside of plotting window [#2284](https://github.com/emilk/egui/pull/2284) (thanks [@JohannesProgrammiert](https://github.com/JohannesProgrammiert)!)
* Add `PlotUi::response()` to replace `plot_clicked()` etc [#3223](https://github.com/emilk/egui/pull/3223)
* Add rotation feature to plot images [#3121](https://github.com/emilk/egui/pull/3121) (thanks [@ThundR67](https://github.com/ThundR67)!)
* Plot items: Image rotation and size in plot coordinates, polygon fill color [#3182](https://github.com/emilk/egui/pull/3182) (thanks [@s-nie](https://github.com/s-nie)!)
* Add method to specify `tip_size` of plot arrows [#3138](https://github.com/emilk/egui/pull/3138) (thanks [@nagua](https://github.com/nagua)!)
* Better handle additive colors in plots [#3387](https://github.com/emilk/egui/pull/3387)
* Fix auto_bounds when only one axis has restricted navigation [#3171](https://github.com/emilk/egui/pull/3171) (thanks [@KoffeinFlummi](https://github.com/KoffeinFlummi)!)
* Fix plot formatter not taking closures [#3260](https://github.com/emilk/egui/pull/3260) (thanks [@Wumpf](https://github.com/Wumpf)!)
