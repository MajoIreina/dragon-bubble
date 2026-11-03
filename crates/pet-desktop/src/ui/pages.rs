//! Settings pages share the same card and row components.
use super::*;

pub(super) fn appearance_page(
    ui: &mut egui::Ui,
    rt: &mut Runtime,
    state: &mut UiState,
    views: &RenderViews,
    textures: &mut EguiUserTextures,
) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        let texture = textures.add_image(EguiTextureHandle::Weak(views.preview.id()));
        let size = (ui.available_width() * 0.34).clamp(100., 210.);
        ui.horizontal(|ui| {
            ui.add(egui::Image::new((texture, egui::vec2(size, size))));
            ui.vertical(|ui| {
                ui.add_space(8.);
                ui.label(
                    egui::RichText::new(tr("选择外观"))
                        .size(18.)
                        .strong()
                        .family(egui::FontFamily::Name("YaHeiBold".into())),
                );
                ui.label(
                    egui::RichText::new(tr("点击外观即可切换，设置会自动保存。"))
                        .small()
                        .weak(),
                );
                ui.add_space(8.);
                for skin in rt.skins.clone() {
                    if ui
                        .selectable_label(rt.prefs.skin_id == skin.id, tr(&skin.name))
                        .clicked()
                    {
                        rt.actions.push(Action::SelectSkin(skin.id));
                    }
                }
            });
        });
    });
    section(ui, &tr("管理外观"));
    setting_row(
        ui,
        "＋",
        &tr("导入外观"),
        &tr("支持 GLB、glTF、VRM、FBX、OBJ、Blender、STL、PLY 和图片外观。"),
        120.,
        |ui| {
            if ui
                .add_enabled(
                    rt.importing.is_none() && state.picker.is_none(),
                    egui::Button::new(tr("选择文件")),
                )
                .clicked()
            {
                open_picker(state, false);
            }
        },
    );
    setting_row(
        ui,
        "▱",
        &tr("外观文件夹"),
        &tr("查看和管理已导入的外观文件"),
        120.,
        |ui| {
            if ui.button(tr("打开目录")).clicked() {
                let dir = rt.data.join("skins");
                let _ = std::fs::create_dir_all(&dir);
                let opener = if cfg!(target_os = "macos") {
                    "open"
                } else {
                    "explorer.exe"
                };
                let _ = std::process::Command::new(opener).arg(dir).spawn();
            }
        },
    );
    for error in &rt.diagnostics {
        ui.colored_label(error_color(ui), pet_core::i18n::message(&locale, error));
    }
}
pub(super) fn behavior_page(ui: &mut egui::Ui, rt: &mut Runtime) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    let mut roaming = rt.prefs.roaming;
    setting_row(
        ui,
        "♡",
        &tr("自由走动"),
        &tr("允许桌宠在屏幕上散步、奔跑和休息"),
        80.,
        |ui| {
            if toggle(ui, &mut roaming, &tr("自由走动")).changed() {
                rt.actions.push(Action::Roam(roaming));
            }
        },
    );
    setting_row(
        ui,
        "◷",
        &tr("当前状态"),
        &tr("桌宠会根据互动和时间自然切换状态"),
        120.,
        |ui| {
            ui.label(tr(rt.brain.state.label()));
        },
    );
    let sleeping = rt.brain.forced_sleep || rt.brain.state == State::Sleep;
    setting_row(
        ui,
        "☾",
        &tr("睡眠"),
        &tr("一键睡觉后持续休息，点击、拖动或唤醒即可结束"),
        110.,
        |ui| {
            if ui
                .button(tr(if sleeping { "唤醒" } else { "一键睡觉" }))
                .clicked()
            {
                rt.actions.push(Action::Sleep(!sleeping));
            }
        },
    );
    setting_row(
        ui,
        "▣",
        &tr("桌面窗口捕捉"),
        &tr("落在可见窗口顶边；窗口关闭或最小化后自然下落"),
        80.,
        |ui| {
            toggle(
                ui,
                &mut rt.prefs.behavior.window_perching,
                &tr("桌面窗口捕捉"),
            );
        },
    );
    setting_row(
        ui,
        "↻",
        &tr("待机原地转圈"),
        &tr("待机时偶尔转一圈，保持原地站立"),
        80.,
        |ui| {
            toggle(ui, &mut rt.prefs.behavior.idle_spin, &tr("待机原地转圈"));
        },
    );
    section(ui, &tr("动作时间"));
    card(ui).show(ui, |ui| {
        ui.label(tr("各动作持续时间（秒），自动活动有少量随机变化"));
        for (label, value, range) in [
            ("待机间隔", &mut rt.prefs.behavior.idle, 3.0..=120.0),
            ("走路时长", &mut rt.prefs.behavior.walk, 1.0..=30.0),
            ("跑步时长", &mut rt.prefs.behavior.run, 0.5..=15.0),
            ("自动睡眠时长", &mut rt.prefs.behavior.sleep, 5.0..=600.0),
            ("转圈间隔", &mut rt.prefs.behavior.spin_interval, 4.0..=60.0),
        ] {
            ui.add(
                egui::Slider::new(value, range)
                    .text(tr(label))
                    .suffix(" s")
                    .step_by(0.5),
            );
        }
        if ui.button(tr("恢复默认动作时间")).clicked() {
            let options = &mut rt.prefs.behavior;
            *options = pet_core::behavior::BehaviorOptions {
                idle_spin: options.idle_spin,
                window_perching: options.window_perching,
                ..Default::default()
            };
        }
    });
    section(ui, &tr("互动指南"));
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        for (title, description) in [
            ("点击", "点击：摸摸回应"),
            ("拖动", "拖动：提起桌宠"),
            ("松手", "松手：惯性抛掷、重力与落地反弹"),
            ("右键", "右键：打开快捷菜单"),
        ] {
            ui.horizontal_wrapped(|ui| {
                ui.add_sized(
                    [58., 28.],
                    egui::Label::new(
                        egui::RichText::new(tr(title))
                            .strong()
                            .family(egui::FontFamily::Name("YaHeiBold".into())),
                    ),
                );
                ui.label(egui::RichText::new(tr(description)).weak());
            });
        }
        ui.separator();
        ui.label(
            egui::RichText::new(tr("关闭设置窗口后，桌宠继续在托盘运行。"))
                .small()
                .weak(),
        );
    });
}
pub(super) fn display_page(ui: &mut egui::Ui, rt: &mut Runtime) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    let mut top = rt.prefs.always_on_top;
    setting_row(
        ui,
        "▣",
        &tr("始终置顶"),
        &tr("让桌宠显示在其他普通窗口上方"),
        80.,
        |ui| {
            if toggle(ui, &mut top, &tr("始终置顶")).changed() {
                rt.actions.push(Action::Top(top));
            }
        },
    );
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(tr("桌宠大小"));
        ui.label(
            egui::RichText::new(tr("鼠标悬停在桌宠上滚动滚轮，或拖动滑条调整"))
                .small()
                .weak(),
        );
        ui.add_space(8.);
        scale_control(ui, rt);
    });
    section(ui, &tr("屏幕与位置"));
    for (i, area) in rt.desktop.areas.clone().iter().enumerate() {
        setting_row(
            ui,
            "▣",
            &format!("{} {}", tr("屏幕"), i + 1),
            &format!(
                "{} × {} · ({}, {})",
                area.size().x,
                area.size().y,
                area.min.x,
                area.min.y
            ),
            120.,
            |ui| {
                if ui.button(tr("移动到此屏幕")).clicked() {
                    rt.actions.push(Action::MoveScreen(i));
                }
            },
        );
    }
    setting_row(
        ui,
        "↺",
        &tr("重置位置"),
        &tr("将桌宠找回到当前可用的屏幕内"),
        100.,
        |ui| {
            if ui.button(tr("重置")).clicked() {
                rt.actions.push(Action::Reset);
            }
        },
    );
}
pub(super) fn adaptation_page(
    ui: &mut egui::Ui,
    rt: &mut Runtime,
    state: &mut UiState,
    views: &RenderViews,
    textures: &mut EguiUserTextures,
    avatars: &Query<&Avatar>,
) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        let texture = textures.add_image(EguiTextureHandle::Weak(views.preview.id()));
        let size = (ui.available_width() * 0.32).clamp(100., 180.);
        ui.horizontal(|ui| {
            ui.add(egui::Image::new((texture, egui::vec2(size, size))));
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(tr("动作预览"))
                        .size(18.)
                        .strong()
                        .family(egui::FontFamily::Name("YaHeiBold".into())),
                );
                ui.label(
                    egui::RichText::new(tr("选择状态以查看对应动作"))
                        .small()
                        .weak(),
                );
                ui.add_space(8.);
                ui.horizontal_wrapped(|ui| {
                    for action in State::ALL {
                        if ui
                            .selectable_label(rt.preview_state == action, tr(action.label()))
                            .clicked()
                        {
                            rt.actions.push(Action::Preview(action));
                        }
                    }
                });
            });
        });
    });
    if state.adaptation_id != rt.prefs.skin_id || state.adaptation.is_none() {
        state.adaptation_id = rt.prefs.skin_id.clone();
        state.adaptation = rt.effective_skin(&rt.prefs.skin_id);
        state.converter =
            std::fs::read_to_string(rt.data.join("converter-path.txt")).unwrap_or_default();
    }
    let clips = rt
        .active
        .and_then(|e| avatars.get(e).ok())
        .map(|a| {
            let mut keys = a.clips.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        })
        .unwrap_or_default();
    section(ui, &tr("模型适配"));
    if let Some(skin) = state.adaptation.as_mut() {
        let previous = serde_json::to_value(&skin).ok();
        setting_row(
            ui,
            "◇",
            &tr("动作类型"),
            &tr("未映射的状态使用对应风格的通用动作"),
            210.,
            |ui| {
                choice(
                    ui,
                    &locale,
                    "动作类型",
                    &mut skin.profile,
                    &[
                        ("auto", "自动识别"),
                        ("blob", "龙泡泡 / 软团"),
                        ("character", "Q 版人物"),
                        ("flat", "图片 / 平面角色"),
                    ],
                );
            },
        );
        setting_row(
            ui,
            "↻",
            &tr("模型朝向"),
            &tr("调整模型三个坐标轴的旋转角度"),
            250.,
            |ui| {
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 6.;
                        for (i, axis) in ["X", "Y", "Z"].iter().enumerate() {
                            ui.add(
                                egui::DragValue::new(&mut skin.rotation_degrees[i])
                                    .speed(1.)
                                    .range(-360.0..=360.0)
                                    .prefix(format!("{axis} ")),
                            );
                        }
                    },
                );
            },
        );
        setting_row(
            ui,
            "↔",
            &tr("随方向翻转"),
            &tr("平面外观跟随移动方向左右翻转"),
            80.,
            |ui| {
                toggle(ui, &mut skin.flip_with_direction, &tr("随方向翻转"));
            },
        );
        section(ui, &tr("状态动作映射"));
        card(ui).show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            egui::Grid::new("animation-mapping")
                .num_columns(2)
                .spacing([24., 12.])
                .show(ui, |ui| {
                    for action in State::ALL {
                        ui.label(tr(action.label()));
                        let mut mapped = skin
                            .animations
                            .get(action.key())
                            .cloned()
                            .unwrap_or("__auto".into());
                        egui::ComboBox::from_id_salt(action.key())
                            .width((ui.available_width() - 24.).clamp(160., 380.))
                            .selected_text(if mapped == "__auto" {
                                tr("自动识别")
                            } else if mapped.is_empty() {
                                tr("通用动作")
                            } else {
                                mapped.clone()
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut mapped, "__auto".into(), tr("自动识别"));
                                ui.selectable_value(&mut mapped, String::new(), tr("通用动作"));
                                for clip in &clips {
                                    ui.selectable_value(&mut mapped, clip.clone(), clip);
                                }
                            });
                        if mapped == "__auto" {
                            skin.animations.remove(action.key());
                        } else {
                            skin.animations.insert(action.key().into(), mapped);
                        }
                        ui.end_row();
                    }
                });
        });
        if previous != serde_json::to_value(&skin).ok() {
            state.adaptation_changed = Some(std::time::Instant::now());
        }
    }
    setting_row(
        ui,
        "↺",
        &tr("重置适配"),
        &tr("恢复当前外观的默认动作与朝向"),
        100.,
        |ui| {
            if ui.button(tr("重置")).clicked() {
                rt.actions.push(Action::ResetAdaptation);
                state.adaptation = None;
                state.adaptation_changed = None;
            }
        },
    );
    section(ui, &tr("模型转换器"));
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(tr("Blender 路径"));
        ui.label(
            egui::RichText::new(tr("用于导入 Blender、STL 与 PLY 模型，可按需配置"))
                .small()
                .weak(),
        );
        ui.add_space(6.);
        ui.add(
            egui::TextEdit::singleline(&mut state.converter).desired_width(ui.available_width()),
        );
        ui.horizontal(|ui| {
            if ui
                .add_enabled(state.picker.is_none(), egui::Button::new(tr("浏览")))
                .clicked()
            {
                open_picker(state, true);
            }
            if ui.button(tr("保存")).clicked()
                && let Err(e) = pet_core::preferences::atomic_write(
                    &rt.data.join("converter-path.txt"),
                    state.converter.as_bytes(),
                )
            {
                rt.status = e.to_string();
            }
        });
    });
}
pub(super) fn performance_page(ui: &mut egui::Ui, rt: &mut Runtime, state: &mut UiState) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    setting_row(
        ui,
        "⚙",
        &tr("画质方案"),
        &tr("快速选择适合你的性能与功耗方案"),
        320.,
        |ui| {
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                |ui| {
                    ui.spacing_mut().button_padding.x = 8.;
                    for (key, label) in [
                        ("power", "低"),
                        ("balanced", "中"),
                        ("high", "高"),
                        ("ultra", "超高"),
                        ("extreme", "极致"),
                    ] {
                        let preset = RenderOptions::preset(key);
                        if ui
                            .selectable_label(rt.prefs.performance == preset, tr(label))
                            .clicked()
                        {
                            rt.prefs.performance = preset;
                        }
                    }
                },
            );
        },
    );
    let p = &mut rt.prefs.performance;
    section(ui, &tr("渲染与流畅度"));
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().slider_width = ui.available_width();
        ui.horizontal(|ui| {
            ui.label(tr("渲染分辨率"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(tr(RenderOptions::LEVELS[p.resolution_level()]));
            });
        });
        ui.label(
            egui::RichText::new(tr("更高的分辨率呈现更多细节，也会增加显卡负载。"))
                .small()
                .weak(),
        );
        let mut level = p.resolution_level();
        if ui
            .add(
                egui::Slider::new(&mut level, 0..=4)
                    .show_value(false)
                    .step_by(1.)
                    .custom_formatter(|value, _| {
                        tr(RenderOptions::LEVELS[value.round().clamp(0., 4.) as usize])
                    })
                    .custom_parser(|text| {
                        RenderOptions::LEVELS
                            .iter()
                            .position(|label| tr(label) == text)
                            .map(|n| n as f64)
                    }),
            )
            .changed()
        {
            p.resolution = RenderOptions::RESOLUTIONS[level];
        }
    });
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.spacing_mut().slider_width = ui.available_width();
        let mut unlimited = p.fps == 0;
        if p.fps != 0 {
            state.limited_fps = p.fps;
        }
        if state.limited_fps == 0 {
            state.limited_fps = 60;
        }
        let mut fps = state.limited_fps.clamp(24, 240);
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(tr("帧率上限"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                changed |= ui
                    .add_enabled(
                        !unlimited,
                        egui::DragValue::new(&mut fps)
                            .range(24..=240)
                            .suffix(" FPS"),
                    )
                    .changed();
            });
        });
        ui.label(
            egui::RichText::new(tr("更高的帧率带来更流畅的动作，也会增加功耗。"))
                .small()
                .weak(),
        );
        changed |= ui
            .add_enabled(
                !unlimited,
                egui::Slider::new(&mut fps, 24..=240).show_value(false),
            )
            .changed();
        if changed {
            p.fps = fps;
            state.limited_fps = fps;
        }
        ui.horizontal(|ui| {
            if toggle(ui, &mut unlimited, &tr("不限帧率")).changed() {
                p.fps = if unlimited {
                    0
                } else {
                    state.limited_fps.clamp(24, 240)
                };
            }
            ui.label(tr("不限帧率"));
        });
    });
    section(ui, &tr("图像质量"));
    setting_row(
        ui,
        "◇",
        &tr("抗锯齿"),
        &tr("平滑模型轮廓，减少边缘锯齿与闪烁"),
        150.,
        |ui| {
            choice(
                ui,
                &locale,
                "抗锯齿",
                &mut p.aa,
                &[
                    ("off", "关闭"),
                    ("fxaa", "FXAA"),
                    ("taa", "TAA"),
                    ("msaa2", "MSAA 2×"),
                    ("msaa4", "MSAA 4×"),
                    ("msaa8", "MSAA 8×"),
                ],
            );
        },
    );
    setting_row(
        ui,
        "◈",
        &tr("画质"),
        &tr("调整模型细节与阴影质量"),
        240.,
        |ui| {
            ui.add(
                egui::Slider::new(&mut p.quality, 0..=4)
                    .step_by(1.)
                    .custom_formatter(|n, _| {
                        tr(RenderOptions::LEVELS[n.round().clamp(0., 4.) as usize])
                    })
                    .custom_parser(|text| {
                        RenderOptions::LEVELS
                            .iter()
                            .position(|label| tr(label) == text)
                            .map(|n| n as f64)
                    }),
            );
        },
    );
    let ao_hint = if p.ao && p.aa.starts_with("msaa") {
        "环境遮蔽开启时使用 TAA 平滑边缘。"
    } else {
        "增强接触部位的阴影与立体感"
    };
    setting_row(ui, "◐", &tr("环境遮蔽"), &tr(ao_hint), 80., |ui| {
        toggle(ui, &mut p.ao, &tr("环境遮蔽"));
    });
    setting_row(
        ui,
        "↔",
        &tr("垂直同步"),
        &tr("让呈现节奏与显示器刷新同步"),
        80.,
        |ui| {
            toggle(ui, &mut p.vsync, &tr("垂直同步"));
        },
    );
    ui.add_space(8.);
    ui.label(
        egui::RichText::new(format!(
            "FPS {:.1} · {:.2} ms",
            1. / rt.frame_seconds,
            rt.frame_seconds * 1000.
        ))
        .small()
        .weak(),
    );
    ui.label(
        egui::RichText::new(tr("预览仅在设置窗口可见时更新。"))
            .small()
            .weak(),
    );
}
pub(super) fn other_page(ui: &mut egui::Ui, rt: &mut Runtime, state: &mut UiState) {
    let locale = rt.prefs.locale.clone();
    let tr = |s: &str| pet_core::i18n::tr(&locale, s);
    section(ui, &tr("常规"));
    setting_row(
        ui,
        "文",
        &tr("语言"),
        &tr("设置、托盘与菜单的显示语言，切换后立即生效"),
        180.,
        |ui| {
            let mut language = locale.clone();
            choice(
                ui,
                &locale,
                "语言",
                &mut language,
                &[("zh_CN", "简体中文"), ("ja", "日本語"), ("en", "English")],
            );
            if language != locale {
                rt.actions.push(Action::Locale(language));
            }
        },
    );
    setting_row(
        ui,
        "↗",
        &tr("开机自启"),
        &tr("登录电脑后自动启动龙泡泡"),
        80.,
        |ui| {
            let mut enabled = rt.prefs.autostart;
            if toggle(ui, &mut enabled, &tr("开机自启")).changed() {
                rt.actions.push(Action::Autostart(enabled));
            }
        },
    );
    section(ui, &tr("应用更新"));
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("↻")
                    .size(28.)
                    .color(ui.visuals().hyperlink_color),
            );
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(tr("检查更新"))
                        .size(16.)
                        .strong()
                        .family(egui::FontFamily::Name("YaHeiBold".into())),
                );
                ui.label(
                    egui::RichText::new(format!("{} {}", tr("当前版本"), crate::updates::VERSION))
                        .small()
                        .weak(),
                );
            });
        });
        ui.add_space(8.);
        if state.update_receiver.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr("正在检查更新…"));
            });
        } else if let Some(outcome) = &state.update_outcome {
            use crate::updates::Outcome;
            let text = match outcome {
                Outcome::Current => tr("当前已是最新版本"),
                Outcome::Available(release) => tr("发现新版本：{}").replace("{}", &release.version),
                Outcome::Unpublished => tr("项目尚未发布公开版本，可前往发布页面查看。"),
                Outcome::RateLimited => tr("更新服务暂时受限，请稍后重试或查看发布页面。"),
                Outcome::Failed => tr("无法检查更新，请检查网络连接后重试。"),
            };
            let color = if matches!(outcome, Outcome::Failed | Outcome::RateLimited) {
                error_color(ui)
            } else {
                ui.visuals().text_color()
            };
            ui.colored_label(color, text);
            if let Outcome::Available(release) = outcome {
                ui.horizontal_wrapped(|ui| {
                    ui.hyperlink_to(tr("查看此版本"), &release.url);
                    if let Some(url) = &release.download {
                        ui.hyperlink_to(
                            tr(if cfg!(target_os = "macos") {
                                "下载 macOS 版本"
                            } else {
                                "下载 Windows 版本"
                            }),
                            url,
                        );
                    }
                });
                if !release.notes.is_empty() {
                    ui.collapsing(tr("更新说明"), |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(200.)
                            .show(ui, |ui| {
                                ui.label(&release.notes);
                            });
                    });
                }
            }
        } else {
            ui.label(egui::RichText::new(tr("检查 GitHub 上发布的最新正式版本")).weak());
        }
        if let Some(checked) = state.update_checked {
            let minutes = checked.elapsed().unwrap_or_default().as_secs() / 60;
            ui.label(
                egui::RichText::new(if minutes == 0 {
                    tr("刚刚检查")
                } else {
                    tr("上次检查：{} 分钟前").replace("{}", &minutes.to_string())
                })
                .small()
                .weak(),
            );
        }
        ui.add_space(8.);
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    state.update_receiver.is_none(),
                    egui::Button::new(tr("检查更新")),
                )
                .clicked()
            {
                state.check_updates();
            }
            ui.hyperlink_to(tr("前往发布页面"), crate::updates::RELEASES_URL);
            ui.hyperlink_to(tr("GitHub 项目"), crate::updates::PROJECT_URL);
        });
    });
    section(ui, &tr("关于"));
    card(ui).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.horizontal(|ui| {
            if let Some(logo) = &state.logo {
                ui.add(egui::Image::new(logo).fit_to_exact_size(egui::vec2(56., 56.)));
            }
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(tr("龙泡泡"))
                        .size(20.)
                        .strong()
                        .family(egui::FontFamily::Name("YaHeiBold".into())),
                );
                ui.label(egui::RichText::new("My Dragon Bubble").weak());
                ui.label(
                    egui::RichText::new(tr("陪伴在桌面上的小小伙伴"))
                        .small()
                        .weak(),
                );
            });
        });
        ui.add_space(12.);
        for (label, value) in [
            ("版本", crate::updates::VERSION),
            ("作者", "MajoIreina"),
            ("许可证", "MIT"),
        ] {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [90., 24.],
                    egui::Label::new(egui::RichText::new(tr(label)).weak()),
                );
                ui.label(value);
            });
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.hyperlink_to(tr("项目主页"), crate::updates::PROJECT_URL);
            ui.hyperlink_to(
                tr("反馈问题"),
                format!("{}/issues", crate::updates::PROJECT_URL),
            );
        });
    });
}
