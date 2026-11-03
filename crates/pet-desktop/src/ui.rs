mod pages;
use pages::*;

use crate::{
    avatar::{Avatar, RenderViews},
    platform::{SettingsCamera, SettingsWindow},
    runtime::{Action, Runtime},
};
use bevy::{
    camera::RenderTarget,
    camera::visibility::RenderLayers,
    prelude::*,
    window::{WindowRef, WindowResolution},
};
use bevy_egui::{
    EguiContext, EguiGlobalSettings, EguiTextureHandle, EguiUserTextures, PrimaryEguiContext, egui,
};
use pet_core::{behavior::State, preferences::RenderOptions};
type PickerResult = std::sync::Mutex<std::sync::mpsc::Receiver<(bool, Option<std::path::PathBuf>)>>;
type UpdateResult = std::sync::Mutex<std::sync::mpsc::Receiver<crate::updates::Outcome>>;
#[derive(Default)]
pub struct UiState {
    search: String,
    font_loaded: bool,
    logo: Option<egui::TextureHandle>,
    limited_fps: u32,
    adaptation_id: String,
    adaptation: Option<pet_core::assets::Skin>,
    converter: String,
    picker: Option<PickerResult>,
    adaptation_changed: Option<std::time::Instant>,
    update_receiver: Option<UpdateResult>,
    update_outcome: Option<crate::updates::Outcome>,
    update_checked: Option<std::time::SystemTime>,
}
impl UiState {
    fn check_updates(&mut self) {
        if self.update_receiver.is_some() {
            return;
        }
        let (send, receive) = std::sync::mpsc::channel();
        self.update_receiver = Some(std::sync::Mutex::new(receive));
        std::thread::spawn(move || {
            let _ = send.send(crate::updates::check());
        });
    }
}
pub fn setup(mut commands: Commands, mut settings: ResMut<EguiGlobalSettings>, rt: Res<Runtime>) {
    settings.auto_create_primary_context = false;
    let window = commands
        .spawn((
            Window {
                title: "龙泡泡 · My Dragon Bubble".into(),
                window_theme: Some(if crate::platform::prefers_dark_theme() {
                    bevy::window::WindowTheme::Dark
                } else {
                    bevy::window::WindowTheme::Light
                }),
                resolution: WindowResolution::new(1120, 800),
                visible: rt.show_settings,
                resize_constraints: bevy::window::WindowResizeConstraints {
                    min_width: 760.,
                    min_height: 560.,
                    ..default()
                },
                position: bevy::window::WindowPosition::Centered(
                    bevy::window::MonitorSelection::Primary,
                ),
                ..default()
            },
            SettingsWindow,
        ))
        .id();
    commands.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(window)),
            order: 2,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.96, 0.97, 0.99)),
            ..default()
        },
        RenderLayers::layer(3),
        PrimaryEguiContext,
        SettingsCamera,
    ));
}
fn choice(
    ui: &mut egui::Ui,
    locale: &str,
    label: &str,
    selected: &mut String,
    values: &[(&str, &str)],
) {
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width().clamp(100., 220.))
        .wrap_mode(egui::TextWrapMode::Truncate)
        .selected_text(
            values
                .iter()
                .find(|v| v.0 == selected || (selected.is_empty() && v.0 == "auto"))
                .map(|v| pet_core::i18n::tr(locale, v.1))
                .unwrap_or(selected.clone()),
        )
        .show_ui(ui, |ui| {
            for (key, name) in values {
                ui.selectable_value(selected, key.to_string(), pet_core::i18n::tr(locale, name));
            }
        });
}
pub fn draw(
    mut context: Single<&mut EguiContext, With<PrimaryEguiContext>>,
    mut rt: ResMut<Runtime>,
    views: Res<RenderViews>,
    mut textures: ResMut<EguiUserTextures>,
    avatars: Query<&Avatar>,
    mut state: Local<UiState>,
) {
    if rt.check_updates_requested {
        rt.check_updates_requested = false;
        state.check_updates();
    }
    if let Some(outcome) =
        state
            .update_receiver
            .as_ref()
            .and_then(|r| match r.lock().ok()?.try_recv() {
                Ok(outcome) => Some(outcome),
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Some(crate::updates::Outcome::Failed)
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => None,
            })
    {
        state.update_receiver = None;
        state.update_outcome = Some(outcome);
        state.update_checked = Some(std::time::SystemTime::now());
    }
    if let Some((converter, path)) = state
        .picker
        .as_ref()
        .and_then(|r| r.lock().ok()?.try_recv().ok())
    {
        state.picker = None;
        if let Some(path) = path {
            if converter {
                state.converter = path.display().to_string();
            } else {
                rt.actions.push(Action::Import(path));
            }
        }
    }
    if state.adaptation_changed.is_some_and(|t| {
        t.elapsed().as_millis() >= 400
            || state.adaptation_id != rt.prefs.skin_id
            || !rt.show_settings
            || rt.page != 3
            || rt.quit
    }) {
        state.adaptation_changed = None;
        if let Some(skin) = &state.adaptation {
            let value = serde_json::json!({"profile":skin.profile,"rotation_degrees":skin.rotation_degrees,"animations":skin.animations,"flip_with_direction":skin.flip_with_direction});
            match pet_core::preferences::save_override(
                &skin.id,
                Some(value),
                &rt.data.join("appearance-overrides.json"),
            ) {
                Ok(()) => {
                    if skin.id == rt.prefs.skin_id {
                        rt.actions.push(Action::SelectSkin(skin.id.clone()));
                    }
                }
                Err(e) => rt.status = e.to_string(),
            }
        }
    }
    if !rt.show_settings {
        return;
    }
    let ctx = context.get_mut();
    if !state.font_loaded {
        state.font_loaded = true;
        let mut fonts = egui::FontDefinitions::default();
        let mut system_fonts = Vec::new();
        for file in [
            "C:/Windows/Fonts/msyh.ttc",
            "C:/Windows/Fonts/segoeui.ttf",
            "C:/Windows/Fonts/YuGothM.ttc",
            "C:/Windows/Fonts/simhei.ttf",
        ] {
            if let Ok(bytes) = std::fs::read(file) {
                fonts.font_data.insert(
                    file.into(),
                    std::sync::Arc::new(egui::FontData::from_owned(bytes)),
                );
                system_fonts.push(file.to_string());
            }
        }
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .splice(0..0, system_fonts);
        let mut bold = fonts.families[&egui::FontFamily::Proportional].clone();
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/msyhbd.ttc") {
            fonts.font_data.insert(
                "yahei-bold".into(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            bold.insert(0, "yahei-bold".into());
        }
        fonts
            .families
            .insert(egui::FontFamily::Name("YaHeiBold".into()), bold);
        ctx.set_fonts(fonts);
        apply_theme(ctx, crate::platform::prefers_dark_theme());
        if let Ok(logo) = image::load_from_memory(include_bytes!("../../../assets/tray.ico")) {
            let logo = logo.to_rgba8();
            state.logo = Some(ctx.load_texture(
                "shu-logo",
                egui::ColorImage::from_rgba_unmultiplied(
                    [logo.width() as usize, logo.height() as usize],
                    logo.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            ));
        }
        // egui installs new font families at the beginning of the next pass.
        // Drawing named bold fonts in this pass would access the previous atlas.
        ctx.request_repaint();
        return;
    }
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    let before = rt.prefs.clone();
    let dark = ctx.style().visuals.dark_mode;
    let background = if dark {
        egui::Color32::from_rgb(32, 32, 32)
    } else {
        egui::Color32::from_rgb(243, 243, 243)
    };
    egui::SidePanel::left("navigation")
        .exact_width(if ctx.content_rect().width() < 900. {
            208.
        } else {
            244.
        })
        .resizable(false)
        .frame(egui::Frame::new().fill(background).inner_margin(16))
        .show(ctx, |ui| {
            ui.add_space(12.);
            ui.horizontal(|ui| {
                if let Some(logo) = &state.logo {
                    ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(48., 48.)));
                }
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(tr("龙泡泡"))
                                .size(18.)
                                .strong()
                                .family(egui::FontFamily::Name("YaHeiBold".into())),
                        )
                        .truncate(),
                    );
                    ui.add(
                        egui::Label::new(egui::RichText::new("My Dragon Bubble").size(11.).weak())
                            .truncate(),
                    );
                });
            });
            ui.add_space(30.);
            // Search results live in the content pane; navigation stays stable.
            ui.spacing_mut().item_spacing.y = 4.;
            for (i, page) in PAGES.iter().enumerate() {
                if navigation_item(ui, i, state.search.is_empty() && rt.page == i, &tr(page))
                    .clicked()
                {
                    rt.page = i;
                    state.search.clear();
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.add_space(8.);
                if ui
                    .add_sized([ui.available_width(), 32.], egui::Button::new(tr("退出")))
                    .clicked()
                {
                    rt.actions.push(Action::Quit);
                }
                if ui
                    .add_sized(
                        [ui.available_width(), 32.],
                        egui::Button::new(tr("隐藏窗口")),
                    )
                    .clicked()
                {
                    rt.show_settings = false;
                }
                ui.add_space(12.);
                ui.label(
                    egui::RichText::new(format!("v{}", crate::updates::VERSION))
                        .small()
                        .weak(),
                );
            });
        });
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(background)
                .inner_margin(egui::Margin {
                    left: 28,
                    right: 28,
                    top: 14,
                    bottom: 20,
                }),
        )
        .show(ctx, |ui| {
            ui.set_max_width(ui.available_width().min(1080.));
            ui.horizontal(|ui| {
                let search_width = (ui.available_width()
                    - if state.search.is_empty() { 0. } else { 80. })
                .min(520.);
                let response = ui.add_sized(
                    [search_width, 34.],
                    egui::TextEdit::singleline(&mut state.search)
                        .hint_text(tr("查找设置"))
                        .margin(egui::vec2(12., 8.)),
                );
                if response.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    state.search.clear();
                    response.surrender_focus();
                }
                if !state.search.is_empty() && ui.small_button(tr("清除")).clicked() {
                    state.search.clear();
                }
            });
            ui.add_space(22.);
            rt.page = rt.page.min(PAGES.len() - 1);
            ui.label(
                egui::RichText::new(tr(if state.search.is_empty() {
                    PAGES[rt.page]
                } else {
                    "搜索结果"
                }))
                .size(28.)
                .strong()
                .family(egui::FontFamily::Name("YaHeiBold".into())),
            );
            ui.label(
                egui::RichText::new(tr(if state.search.is_empty() {
                    DESCRIPTIONS[rt.page]
                } else {
                    "选择结果以打开对应的设置页面"
                }))
                .weak(),
            );
            ui.add_space(20.);
            egui::ScrollArea::vertical()
                .id_salt(("page", rt.page, !state.search.is_empty()))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().slider_width = (ui.available_width() - 120.).clamp(160., 460.);
                    if !state.search.is_empty() {
                        let results = search_pages(&state.search, &locale);
                        if results.is_empty() {
                            setting_row(
                                ui,
                                "⌕",
                                &tr("未找到相关设置"),
                                &tr("试试其他关键词，例如语言、更新、帧率或外观。"),
                                0.,
                                |_| {},
                            );
                        }
                        for i in results {
                            let mut open = false;
                            setting_row(
                                ui,
                                ICONS[i],
                                &tr(PAGES[i]),
                                &tr(DESCRIPTIONS[i]),
                                70.,
                                |ui| {
                                    open = ui.button(tr("打开")).clicked();
                                },
                            );
                            if open {
                                rt.page = i;
                                state.search.clear();
                            }
                        }
                    } else {
                        match rt.page {
                            0 => appearance_page(ui, &mut rt, &mut state, &views, &mut textures),
                            1 => behavior_page(ui, &mut rt),
                            2 => display_page(ui, &mut rt),
                            3 => adaptation_page(
                                ui,
                                &mut rt,
                                &mut state,
                                &views,
                                &mut textures,
                                &avatars,
                            ),
                            4 => performance_page(ui, &mut rt, &mut state),
                            _ => other_page(ui, &mut rt, &mut state),
                        }
                    }
                    if !rt.status.is_empty() {
                        ui.add_space(8.);
                        ui.colored_label(
                            error_color(ui),
                            pet_core::i18n::message(&locale, &rt.status),
                        );
                    }
                    if rt.importing.is_some() {
                        ui.spinner();
                    }
                    ui.add_space(8.);
                });
        });
    if rt.prefs != before {
        rt.dirty = true;
    }
}

fn scale_control(ui: &mut egui::Ui, rt: &mut Runtime) {
    let tr = |s: &str| pet_core::i18n::tr(&rt.prefs.locale, s);
    let mut percent = rt.prefs.zoom * 100.;
    let slider = ui
        .add(
            egui::Slider::new(&mut percent, 20.0..=300.0)
                .step_by(1.)
                .show_value(false)
                .text(tr("大小")),
        )
        .changed();
    let mut edited = false;
    let mut reset = false;
    ui.horizontal(|ui| {
        edited = ui
            .add(
                egui::DragValue::new(&mut percent)
                    .range(20.0..=300.0)
                    .speed(1.)
                    .suffix("%"),
            )
            .changed();
        reset = ui.button(tr("恢复 100%")).clicked();
    });
    if reset {
        rt.actions.push(Action::Scale(1.));
    } else if slider || edited {
        rt.actions.push(Action::Scale(percent / 100.));
    }
}
const PAGES: [&str; 6] = ["外观", "行为", "显示", "动作与适配", "性能与画质", "其他"];
const DESCRIPTIONS: [&str; 6] = [
    "为桌面选择一位伙伴",
    "让每一次互动都有回应",
    "适合你的桌面与屏幕",
    "预览动作，适配你的模型",
    "平衡清晰度、流畅度与功耗",
    "语言、更新与应用信息",
];
const ICONS: [&str; 6] = ["◇", "♡", "▣", "▷", "⚙", "…"];
fn search_pages(query: &str, locale: &str) -> Vec<usize> {
    let query = query.trim().to_lowercase();
    let terms = [
        "skin appearance import 外观 模型 导入 画像 スキン",
        "roam behavior sleep spin interval window 行为 走动 睡眠 睡觉 唤醒 互动 转圈 间隔 时间 窗口 捕捉 動作",
        "window scale monitor display top 显示 大小 置顶 屏幕 位置 画面",
        "animation mapping converter blender 动作 适配 转换 朝向 翻转 アニメーション",
        "fps aa taa resolution quality performance vsync 性能 画质 帧率 分辨率 抗锯齿 同步 環境遮蔽",
        "other language update about version その他 言語 更新 バージョン 其他 语言 关于 版本 简体中文 english 日本語",
    ];
    (0..PAGES.len())
        .filter(|&i| {
            query.split_whitespace().all(|word| {
                terms[i].contains(word)
                    || pet_core::i18n::tr(locale, PAGES[i])
                        .to_lowercase()
                        .contains(word)
                    || pet_core::i18n::tr(locale, DESCRIPTIONS[i])
                        .to_lowercase()
                        .contains(word)
            })
        })
        .collect()
}
fn apply_theme(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    style.visuals.override_text_color = Some(if dark {
        egui::Color32::from_gray(245)
    } else {
        egui::Color32::from_gray(28)
    });
    style.visuals.weak_text_color = Some(if dark {
        egui::Color32::from_gray(184)
    } else {
        egui::Color32::from_gray(98)
    });
    let accent = if dark {
        egui::Color32::from_rgb(96, 205, 255)
    } else {
        egui::Color32::from_rgb(0, 103, 192)
    };
    style.visuals.selection.bg_fill = if dark {
        egui::Color32::from_rgb(35, 72, 92)
    } else {
        egui::Color32::from_rgb(218, 235, 249)
    };
    style.visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent);
    style.visuals.hyperlink_color = accent;
    style.visuals.window_corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(
        1.0_f32,
        if dark {
            egui::Color32::from_gray(60)
        } else {
            egui::Color32::from_gray(225)
        },
    );
    style.visuals.extreme_bg_color = if dark {
        egui::Color32::from_gray(38)
    } else {
        egui::Color32::from_gray(250)
    };
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(5);
        widget.bg_stroke = egui::Stroke::new(
            1.0_f32,
            if dark {
                egui::Color32::from_gray(76)
            } else {
                egui::Color32::from_gray(211)
            },
        );
    }
    style.visuals.widgets.inactive.weak_bg_fill = if dark {
        egui::Color32::from_gray(55)
    } else {
        egui::Color32::from_gray(252)
    };
    style.visuals.widgets.hovered.weak_bg_fill = if dark {
        egui::Color32::from_gray(65)
    } else {
        egui::Color32::from_gray(246)
    };
    style.visuals.widgets.active.weak_bg_fill = if dark {
        egui::Color32::from_gray(48)
    } else {
        egui::Color32::from_gray(238)
    };
    style.visuals.widgets.inactive.bg_fill = if dark {
        egui::Color32::from_gray(115)
    } else {
        egui::Color32::from_gray(170)
    };
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.5_f32, accent);
    style.visuals.slider_trailing_fill = true;
    style.visuals.handle_shape = egui::style::HandleShape::Circle;
    style.spacing.slider_rail_height = 4.;
    style.spacing.item_spacing = egui::vec2(12., 8.);
    style.spacing.button_padding = egui::vec2(16., 7.);
    style.spacing.interact_size.y = 32.;
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
    ctx.set_style(style);
}
fn card(ui: &egui::Ui) -> egui::Frame {
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(if dark {
            egui::Color32::from_gray(43)
        } else {
            egui::Color32::from_gray(252)
        })
        .stroke(egui::Stroke::new(
            1.0_f32,
            if dark {
                egui::Color32::from_gray(52)
            } else {
                egui::Color32::from_gray(229)
            },
        ))
        .corner_radius(7)
        .inner_margin(18)
}
fn error_color(ui: &egui::Ui) -> egui::Color32 {
    if ui.visuals().dark_mode {
        egui::Color32::from_rgb(255, 170, 170)
    } else {
        egui::Color32::from_rgb(174, 34, 34)
    }
}
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(10.);
    ui.label(
        egui::RichText::new(title)
            .size(15.)
            .strong()
            .family(egui::FontFamily::Name("YaHeiBold".into())),
    );
    ui.add_space(4.);
}
fn setting_row(
    ui: &mut egui::Ui,
    icon: &str,
    title: &str,
    description: &str,
    control_width: f32,
    control: impl FnOnce(&mut egui::Ui),
) {
    let mut control = Some(control);
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().slider_width = (control_width - 90.).max(90.);
        let stacked = ui.available_width() < 540. && control_width > 140.;
        ui.horizontal(|ui| {
            ui.add_sized(
                [28., 32.],
                egui::Label::new(
                    egui::RichText::new(icon)
                        .size(22.)
                        .color(ui.visuals().hyperlink_color),
                ),
            );
            let text_width =
                (ui.available_width() - if stacked { 0. } else { control_width + 12. }).max(90.);
            ui.allocate_ui_with_layout(
                egui::vec2(text_width, 32.),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(text_width);
                    ui.spacing_mut().item_spacing.y = 3.;
                    ui.add(egui::Label::new(egui::RichText::new(title).size(14.)).wrap());
                    if !description.is_empty() {
                        ui.add(
                            egui::Label::new(egui::RichText::new(description).small().weak())
                                .wrap(),
                        );
                    }
                },
            );
            if !stacked {
                ui.allocate_ui_with_layout(
                    egui::vec2(control_width, 32.),
                    egui::Layout::right_to_left(egui::Align::Center),
                    control.take().expect("control used once"),
                );
            }
        });
        if stacked {
            ui.add_space(8.);
            ui.horizontal(|ui| {
                ui.add_space(40.);
                control.take().expect("stacked control")(ui);
            });
        }
    });
}
fn toggle(ui: &mut egui::Ui, value: &mut bool, label: &str) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(40., 21.), egui::Sense::click());
    if response.clicked() {
        *value = !*value;
        response.mark_changed();
    }
    let accent = ui.visuals().hyperlink_color;
    let fill = if *value {
        accent
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect(
        rect,
        11.,
        fill,
        egui::Stroke::new(
            1.0_f32,
            if *value {
                accent
            } else {
                ui.visuals().weak_text_color()
            },
        ),
        egui::StrokeKind::Inside,
    );
    let x = if *value {
        rect.right() - 10.
    } else {
        rect.left() + 10.
    };
    ui.painter().circle_filled(
        egui::pos2(x, rect.center().y),
        if response.hovered() { 7. } else { 6. },
        if *value {
            if ui.visuals().dark_mode {
                egui::Color32::from_gray(28)
            } else {
                egui::Color32::WHITE
            }
        } else {
            ui.visuals().weak_text_color()
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *value, label)
    });
    response.on_hover_text(label)
}
fn navigation_item(ui: &mut egui::Ui, index: usize, selected: bool, label: &str) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.), egui::Sense::click());
    if selected || response.hovered() {
        let gray = if ui.visuals().dark_mode {
            if selected { 47 } else { 42 }
        } else if selected {
            229
        } else {
            234
        };
        ui.painter()
            .rect_filled(rect, 5., egui::Color32::from_gray(gray));
    }
    if selected {
        ui.painter().rect_filled(
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 2., rect.center().y),
                egui::vec2(3., 16.),
            ),
            2.,
            ui.visuals().hyperlink_color,
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 24., rect.center().y),
        egui::Align2::CENTER_CENTER,
        ICONS[index],
        egui::FontId::proportional(21.),
        ui.visuals().hyperlink_color,
    );
    ui.painter().text(
        egui::pos2(rect.left() + 48., rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::new(14., egui::FontFamily::Name("YaHeiBold".into())),
        ui.visuals().text_color(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, selected, label)
    });
    response
}

fn open_picker(state: &mut UiState, converter: bool) {
    let (send, receive) = std::sync::mpsc::channel();
    state.picker = Some(std::sync::Mutex::new(receive));
    std::thread::spawn(move || {
        let dialog = rfd::FileDialog::new();
        let path = if converter {
            dialog.add_filter("Blender", &["exe"]).pick_file()
        } else {
            dialog
                .add_filter(
                    "Skin / Model / Image",
                    &[
                        "json", "glb", "gltf", "vrm", "fbx", "obj", "blend", "stl", "ply", "png",
                        "jpg", "jpeg", "webp", "bmp", "tga", "svg", "gif",
                    ],
                )
                .pick_file()
        };
        let _ = send.send((converter, path));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_includes_new_page_and_keeps_existing_setting_keywords() {
        for term in [
            "语言",
            "更新",
            "关于",
            "language",
            "UPDATE",
            "日本語",
            "言語",
        ] {
            assert!(search_pages(term, "zh_CN").contains(&5), "{term}");
        }
        assert_eq!(search_pages("fps", "en"), vec![4]);
        assert_eq!(search_pages("置顶", "zh_CN"), vec![2]);
        assert!(search_pages("not-a-setting", "en").is_empty());
    }
}
