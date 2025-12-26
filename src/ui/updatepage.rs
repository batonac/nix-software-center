use crate::{APPINFO, ui::unavailabledialog::UnavailableDialogModel, parse::util};

use super::{pkgpage::InstallType, window::*, updateworker::{UpdateAsyncHandler, UpdateAsyncHandlerMsg, UpdateAsyncHandlerInit}, unavailabledialog::UnavailableDialogMsg};
use adw::prelude::*;
use relm4::{factory::*, gtk::pango, *};
use std::{convert::identity, collections::HashMap};
use log::*;

pub static UNAVAILABLE_BROKER: MessageBroker<UnavailableDialogMsg> = MessageBroker::new();

#[tracker::track]
#[derive(Debug)]
pub struct UpdatePageModel {
    #[tracker::no_eq]
    updateuserlist: FactoryVecDeque<UpdateItemModel>,
    #[tracker::no_eq]
    updateworker: WorkerController<UpdateAsyncHandler>,
    updatetracker: u8,
    #[tracker::no_eq]
    unavailabledialog: Controller<UnavailableDialogModel>,
    online: bool,
}

#[derive(Debug)]
pub enum UpdatePageMsg {
    Update(Vec<UpdateItem>, Vec<UpdateItem>),
    OpenRow(usize, InstallType),
    UpdateAllUser,
    UpdateAllUserRm(Vec<String>),
    UpdateUser(String),
    DoneWorking,
    FailedWorking,
    UpdateOnline(bool),
}

#[derive(Debug)]
pub enum UpdateType {
    User,
}

pub struct UpdatePageInit {
    pub window: gtk::Window,
    pub online: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for UpdatePageModel {
    type Init = UpdatePageInit;
    type Input = UpdatePageMsg;
    type Output = AppMsg;

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            #[track(model.changed(UpdatePageModel::updatetracker()))]
            set_vadjustment: gtk::Adjustment::NONE,
            adw::Clamp {
                #[name(mainstack)]
                if !model.online {
                    adw::StatusPage {
                        set_icon_name: Some("nsc-network-offline-symbolic"),
                        set_title: "No internet connection",
                        set_description: Some("Please connect to the internet to update your system"),
                        gtk::Button {
                            add_css_class: "pill",
                            set_halign: gtk::Align::Center,
                            adw::ButtonContent {
                                set_icon_name: "nsc-refresh-symbolic",
                                set_label: "Refresh",
                            },
                            connect_clicked[sender] => move |_| {
                                sender.output(AppMsg::CheckNetwork);
                            }
                        }
                    }
                } else if !model.updateuserlist.is_empty() {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Start,
                        set_margin_all: 15,
                        set_spacing: 15,
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-2",
                                set_label: "Updates",
                            },
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: "Update All User Packages",
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAllUser);
                                }
                            }
                        },
                        gtk::Box {
                            set_orientation: gtk::Orientation::Horizontal,
                            set_hexpand: true,
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                add_css_class: "title-4",
                                set_label: "User (nix profile)"
                            gtk::Button {
                                add_css_class: "suggested-action",
                                set_halign: gtk::Align::End,
                                set_valign: gtk::Align::Center,
                                set_hexpand: true,
                                set_label: "Update All",
                                connect_clicked[sender] => move |_| {
                                    sender.input(UpdatePageMsg::UpdateAllUser);
                                }
                            }
                        },
                        #[local_ref]
                        updateuserlist -> gtk::ListBox {
                            set_valign: gtk::Align::Start,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                            connect_row_activated[sender] => move |listbox, row| {
                                if let Some(i) = listbox.index_of_child(row) {
                                    sender.input(UpdatePageMsg::OpenRow(i as usize, InstallType::User));
                                }
                            },
                            #[watch]
                            set_visible: !model.updateuserlist.is_empty(),
                        }
                    }
                } else {
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_valign: gtk::Align::Center,
                        set_halign: gtk::Align::Center,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_spacing: 10,
                        gtk::Image {
                            add_css_class: "success",
                            set_icon_name: Some("emblem-ok-symbolic"),
                            set_pixel_size: 256,
                        },
                        gtk::Label {
                            add_css_class: "title-1",
                            set_label: "Everything is up to date!"
                        }
                    }
                }
            }
        }
    }

    fn init(
        initparams: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let updateworker = UpdateAsyncHandler::builder()
            .detach_worker(UpdateAsyncHandlerInit {})
            .forward(sender.input_sender(), identity);

        let unavailabledialog = UnavailableDialogModel::builder()
            .launch_with_broker(initparams.window.clone(), &UNAVAILABLE_BROKER)
            .forward(sender.input_sender(), identity);

        let model = UpdatePageModel {
            updateuserlist: FactoryVecDeque::builder().launch(gtk::ListBox::new()).detach(),
            updatetracker: 0,
            updateworker,
            unavailabledialog,
            online: initparams.online,
            tracker: 0,
        };

        let updateuserlist = model.updateuserlist.widget();

        let widgets = view_output!();
        widgets.mainstack.set_hhomogeneous(false);
        widgets.mainstack.set_vhomogeneous(false);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            UpdatePageMsg::Update(updateuserlist, _updatesystemlist) => {
                info!("UpdatePageMsg::Update");
                debug!("UPDATEUSERLIST: {:?}", updateuserlist);
                self.update_updatetracker(|_| ());
                let mut updateuserlist_guard = self.updateuserlist.guard();
                updateuserlist_guard.clear();
                for updateuser in updateuserlist {
                    updateuserlist_guard.push_back(updateuser);
                }
            }
            UpdatePageMsg::OpenRow(row, pkgtype) => match pkgtype {
                InstallType::User => {
                    let updateuserlist_guard = self.updateuserlist.guard();
                    if let Some(item) = updateuserlist_guard.get(row) {
                        if let Some(pkg) = &item.item.pkg {
                            sender.output(AppMsg::OpenPkg(pkg.to_string()));
                        }
                    }
                }
                InstallType::System => {
                    // System operations no longer supported
                }
            },
            UpdatePageMsg::UpdateAllUser => {
                let online = util::checkonline();
                if !online {
                    sender.output(AppMsg::CheckNetwork);
                    self.online = false;
                    return;
                }
                let workersender = self.updateworker.sender().clone();
                let output = sender.output_sender().clone();
                relm4::spawn(async move {
                    let uninstalluser = nix_data::cache::profile::unavailablepkgs().await.unwrap_or_default();
                    if uninstalluser.is_empty() {
                        workersender.send(UpdateAsyncHandlerMsg::UpdateUserPkgs);
                    } else {
                        warn!("Uninstalling unavailable packages: {:?}", uninstalluser);
                        output.send(AppMsg::GetUnavailableItems(uninstalluser, HashMap::new(), UpdateType::User));
                    }
                });
            }
            UpdatePageMsg::UpdateAllUserRm(pkgs) => {
                info!("UpdatePageMsg::UpdateAllUserRm({:?})", pkgs);
                self.updateworker.emit(UpdateAsyncHandlerMsg::UpdateUserPkgsRemove(pkgs));
            }
            UpdatePageMsg::DoneWorking => {
                let _ = nix_data::utils::refreshicons();
                sender.output(AppMsg::UpdateInstalledPkgs);
            }
            UpdatePageMsg::FailedWorking => {
                // Update failed
            }
            UpdatePageMsg::UpdateOnline(online) => {
                self.set_online(online);
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateItem {
    pub name: String,
    pub pkg: Option<String>,
    pub pname: String,
    pub summary: Option<String>,
    pub icon: Option<String>,
    pub pkgtype: InstallType,
    pub verfrom: Option<String>,
    pub verto: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct UpdateItemModel {
    item: UpdateItem,
}

#[derive(Debug)]
pub enum UpdateItemMsg {}

#[relm4::factory(pub)]
impl FactoryComponent for UpdateItemModel {
    type CommandOutput = ();
    type Init = UpdateItem;
    type Input = ();
    type Output = UpdateItemMsg;
    type ParentWidget = adw::gtk::ListBox;

    view! {
        adw::PreferencesRow {
            set_activatable: self.item.pkg.is_some(),
            set_can_focus: false,
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_hexpand: true,
                set_spacing: 10,
                set_margin_all: 10,
                adw::Bin {
                    set_valign: gtk::Align::Center,
                    #[wrap(Some)]
                    set_child = if self.item.icon.is_some() {
                        gtk::Image {
                            add_css_class: "icon-dropshadow",
                            set_halign: gtk::Align::Start,
                            set_from_file: {
                                if let Some(i) = &self.item.icon {
                                    let iconpath = format!("{}/icons/nixos/128x128/{}", APPINFO, i);
                                    let iconpath64 = format!("{}/icons/nixos/64x64/{}", APPINFO, i);
                                    if Path::new(&iconpath).is_file() {
                                        Some(iconpath)
                                    } else if Path::new(&iconpath64).is_file() {
                                        Some(iconpath64)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            },
                            set_pixel_size: 64,
                        }
                    } else {
                        gtk::Image {
                            add_css_class: "icon-dropshadow",
                            set_halign: gtk::Align::Start,
                            set_icon_name: Some("package-x-generic"),
                            set_pixel_size: 64,
                        }
                    }
                },
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Fill,
                    set_valign: gtk::Align::Center,
                    set_hexpand: true,
                    set_spacing: 2,
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: self.item.name.as_str(),
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        add_css_class: "dim-label",
                        add_css_class: "caption",
                        set_label: {
                            &(if let Some(old) = &self.item.verfrom {
                                if let Some(new) = &self.item.verto {
                                    format!("{} → {}", old, new)
                                } else {
                                    String::default()
                                }
                            } else {
                                String::default()
                            })
                        },
                        set_visible: self.item.verfrom.is_some() && self.item.verto.is_some(),
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    },
                    gtk::Label {
                        set_halign: gtk::Align::Start,
                        set_label: self.item.summary.as_deref().unwrap_or(""),
                        set_visible: self.item.summary.is_some(),
                        set_ellipsize: pango::EllipsizeMode::End,
                        set_lines: 1,
                        set_wrap: true,
                        set_max_width_chars: 0,
                    },
                },
                // gtk::Button {
                //     set_visible: self.item.pkgtype == InstallType::User,
                //     set_valign: gtk::Align::Center,
                //     set_halign: gtk::Align::End,
                //     set_label: "Update",
                //     set_can_focus: false,
                // }
            }
        }
    }

    fn init_model(
        parent: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        let sum = if let Some(s) = parent.summary {
            let mut sum = s.trim().to_string();
            while sum.contains('\n') {
                sum = sum.replace('\n', " ");
            }
            while sum.contains("  ") {
                sum = sum.replace("  ", " ");
            }
            Some(sum)
        } else {
            None
        };

        let item = UpdateItem {
            name: parent.name,
            pkg: parent.pkg,
            pname: parent.pname,
            summary: sum,
            icon: parent.icon,
            pkgtype: parent.pkgtype,
            verfrom: parent.verfrom,
            verto: parent.verto,
        };

        Self { item }
    }
}
