# Changelog for egui_plot
All notable changes to the `egui_plot` integration will be noted in this file.

This file is updated upon each release.
Changes since the last release can be found at <https://github.com/emilk/egui/compare/latest...HEAD> or by running the `scripts/generate_changelog.py` script.


## 0.27.2 - 2024-04-02
* Allow zoom/pan a plot as long as it contains the mouse cursor [#4292](https://github.com/emilk/egui/pull/4292)
* Prevent plot from resetting one axis while zooming/dragging the other [#4252](https://github.com/emilk/egui/pull/4252) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* egui_plot: Fix the same plot tick label being painted multiple times [#4307](https://github.com/emilk/egui/pull/4307)


## 0.27.1 - 2024-03-29
* Nothing new


## 0.27.0 - 2024-03-26
* Add `sense` option to `Plot` [#4052](https://github.com/emilk/egui/pull/4052) (thanks [@AmesingFlank](https://github.com/AmesingFlank)!)
* Plot widget - allow disabling scroll for x and y separately [#4051](https://github.com/emilk/egui/pull/4051) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Fix panic when the base step size is set to 0 [#4078](https://github.com/emilk/egui/pull/4078) (thanks [@abey79](https://github.com/abey79)!)
* Expose `PlotGeometry` in public API [#4193](https://github.com/emilk/egui/pull/4193) (thanks [@dwuertz](https://github.com/dwuertz)!)


## 0.26.2 - 2024-02-14
* Nothing new


## 0.26.1 - 2024-02-11
* Nothing new


## 0.26.0 - 2024-02-05
* Make `egui_plot::PlotMemory` public [#3871](https://github.com/emilk/egui/pull/3871)
* Customizable spacing of grid and axis label spacing [#3896](https://github.com/emilk/egui/pull/3896)
* Change default plot line thickness from 1.0 to 1.5 [#3918](https://github.com/emilk/egui/pull/3918)
* Automatically expand plot axes thickness to fit their labels [#3921](https://github.com/emilk/egui/pull/3921)
* Plot items now have optional id which is returned in the plot's response when hovered [#3920](https://github.com/emilk/egui/pull/3920) (thanks [@Wumpf](https://github.com/Wumpf)!)
* Parallel tessellation with opt-in `rayon` feature [#3934](https://github.com/emilk/egui/pull/3934)
* Make `egui_plot::PlotItem` a public trait [#3943](https://github.com/emilk/egui/pull/3943)
* Fix clip rect for plot items [#3955](https://github.com/emilk/egui/pull/3955) (thanks [@YgorSouza](https://github.com/YgorSouza)!)


## 0.25.0 - 2024-01-08
* Fix plot auto-bounds unset by default [#3722](https://github.com/emilk/egui/pull/3722) (thanks [@abey79](https://github.com/abey79)!)
* Add methods to zoom a `Plot` programmatically [#2714](https://github.com/emilk/egui/pull/2714) (thanks [@YgorSouza](https://github.com/YgorSouza)!)
* Add a public API for overriding plot legend traces' visibilities [#3534](https://github.com/emilk/egui/pull/3534) (thanks [@jayzhudev](https://github.com/jayzhudev)!)


## 0.24.1 - 2024-12-03
* Fix plot auto-bounds default [#3722](https://github.com/emilk/egui/pull/3722) (thanks [@abey79](https://github.com/abey79)!)


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
