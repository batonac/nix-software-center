use super::pkgpage::{InstallType, PkgAction, PkgMsg, WorkPkg};
use log::*;
use relm4::*;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;

#[tracker::track]
#[derive(Debug)]
pub struct InstallAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
    work: Option<WorkPkg>,
    pid: Option<u32>,
}

#[derive(Debug)]
pub enum InstallAsyncHandlerMsg {
    Process(WorkPkg),
    CancelProcess,
    SetPid(Option<u32>),
}

#[derive(Debug)]
pub struct InstallAsyncHandlerInit {}

impl Worker for InstallAsyncHandler {
    type Init = InstallAsyncHandlerInit;
    type Input = InstallAsyncHandlerMsg;
    type Output = PkgMsg;

    fn init(_params: Self::Init, _sender: relm4::ComponentSender<Self>) -> Self {
        Self {
            process: None,
            work: None,
            pid: None,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.reset();
        match msg {
            InstallAsyncHandlerMsg::Process(work) => {
                if work.block {
                    return;
                }
                match work.pkgtype {
                    InstallType::User => match work.action {
                        PkgAction::Install => {
                            info!("Installing user package: {}", work.pkg);
                            self.process = Some(relm4::spawn(async move {
                                let mut p = tokio::process::Command::new("nix")
                                    .arg("profile")
                                    .arg("install")
                                    .arg(format!("nixpkgs#{}", work.pkg))
                                    .arg("--impure")
                                    .kill_on_drop(true)
                                    .stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .expect("Failed to run nix profile");

                                let stderr = p.stderr.take().unwrap();
                                let reader = tokio::io::BufReader::new(stderr);

                                let mut lines = reader.lines();
                                while let Ok(Some(line)) = lines.next_line().await {
                                    trace!("CAUGHT LINE: {}", line);
                                }

                                match p.wait().await {
                                    Ok(o) => {
                                        if o.success() {
                                            info!("Installed user package: {} success", work.pkg);
                                            sender.output(PkgMsg::FinishedProcess(work));
                                        } else {
                                            warn!("Installed user package: {} failed", work.pkg);
                                            sender.output(PkgMsg::FailedProcess(work));
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Error installing user package: {}", e);
                                        sender.output(PkgMsg::FailedProcess(work));
                                    }
                                }
                            }));
                        }
                        PkgAction::Remove => {
                            info!("Removing user package: {}", work.pkg);
                            self.process = Some(relm4::spawn(async move {
                                let mut p = tokio::process::Command::new("nix")
                                    .arg("profile")
                                    .arg("remove")
                                    .arg(&format!("legacyPackages.x86_64-linux.{}", work.pkg))
                                    .kill_on_drop(true)
                                    .stdout(Stdio::piped())
                                    .stderr(Stdio::piped())
                                    .spawn()
                                    .expect("Failed to run nix profile");

                                let stderr = p.stderr.take().unwrap();
                                let reader = tokio::io::BufReader::new(stderr);

                                let mut lines = reader.lines();
                                while let Ok(Some(line)) = lines.next_line().await {
                                    trace!("CAUGHT LINE: {}", line);
                                }

                                match p.wait().await {
                                    Ok(o) => {
                                        if o.success() {
                                            info!("Removed user package: {} success", work.pkg);
                                            sender.output(PkgMsg::FinishedProcess(work));
                                        } else {
                                            warn!("Removed user package: {} failed", work.pkg);
                                            sender.output(PkgMsg::FailedProcess(work));
                                        }
                                    }
                                    Err(e) => {
                                        warn!("Error removing user package: {}", e);
                                        sender.output(PkgMsg::FailedProcess(work));
                                    }
                                }
                            }));
                        }
                    },
                    InstallType::System => {
                        warn!("System package operations are no longer supported");
                        sender.output(PkgMsg::FailedProcess(work));
                    }
                }
            }
            InstallAsyncHandlerMsg::CancelProcess => {
                if let Some(process) = &self.process {
                    info!("Cancelling process");
                    process.abort();
                }
                self.process = None;
                self.pid = None;
            }
            InstallAsyncHandlerMsg::SetPid(pid) => {
                self.pid = pid;
            }
        }
    }
}
