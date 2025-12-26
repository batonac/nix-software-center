use anyhow::Result;
use log::*;
use relm4::*;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;

use super::updatepage::UpdatePageMsg;

#[tracker::track]
#[derive(Debug)]
pub struct UpdateAsyncHandler {
    #[tracker::no_eq]
    process: Option<JoinHandle<()>>,
}

#[derive(Debug)]
pub enum UpdateAsyncHandlerMsg {
    UpdateUserPkgs,
    UpdateUserPkgsRemove(Vec<String>),
}

pub struct UpdateAsyncHandlerInit {}

impl Worker for UpdateAsyncHandler {
    type Init = UpdateAsyncHandlerInit;
    type Input = UpdateAsyncHandlerMsg;
    type Output = UpdatePageMsg;

    fn init(_params: Self::Init, _sender: relm4::ComponentSender<Self>) -> Self {
        Self {
            process: None,
            tracker: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            UpdateAsyncHandlerMsg::UpdateUserPkgs => {
                info!("UpdateAsyncHandlerMsg::UpdateUserPkgs");
                self.process = Some(relm4::spawn(async move {
                    match updateprofile(None).await {
                        Ok(_) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        Err(e) => {
                            warn!("Update user pkgs failed: {}", e);
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                }));
            }
            UpdateAsyncHandlerMsg::UpdateUserPkgsRemove(pkgs) => {
                info!("UpdateAsyncHandlerMsg::UpdateUserPkgsRemove");
                self.process = Some(relm4::spawn(async move {
                    match updateprofile(Some(pkgs)).await {
                        Ok(_) => {
                            sender.output(UpdatePageMsg::DoneWorking);
                        }
                        Err(e) => {
                            warn!("Update user pkgs remove failed: {}", e);
                            sender.output(UpdatePageMsg::FailedWorking);
                        }
                    }
                }));
            }
        }
    }
}

async fn updateprofile(rmpkgs: Option<Vec<String>>) -> Result<bool> {
    if let Some(rmpkgs) = rmpkgs {
        if !rmpkgs.is_empty() {
            let mut cmd = tokio::process::Command::new("nix")
                .arg("profile")
                .arg("remove")
                .args(
                    &rmpkgs
                        .iter()
                        .map(|x| format!("legacyPackages.x86_64-linux.{}", x))
                        .collect::<Vec<String>>(),
                )
                .arg("--impure")
                .stderr(Stdio::piped())
                .spawn()?;

            let stderr = cmd.stderr.take().unwrap();
            let reader = tokio::io::BufReader::new(stderr);

            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                trace!("CAUGHT NIX PROFILE LINE: {}", line);
            }
            cmd.wait().await?;
        }
    }

    let mut cmd = tokio::process::Command::new("nix")
        .arg("profile")
        .arg("upgrade")
        .arg(".*")
        .arg("--impure")
        .stderr(Stdio::piped())
        .spawn()?;

    let stderr = cmd.stderr.take().unwrap();
    let reader = tokio::io::BufReader::new(stderr);

    let mut lines = reader.lines();
    while let Ok(Some(line)) = lines.next_line().await {
        trace!("CAUGHT NIX PROFILE LINE: {}", line);
    }
    if cmd.wait().await?.success() {
        Ok(true)
    } else {
        Ok(false)
    }
}
