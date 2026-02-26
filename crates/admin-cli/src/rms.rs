/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

pub use args::Cmd;
use librms::RackManagerClientPool;

use crate::cfg::cli_options::CliOptions;
use crate::rms::args::RmsAction;

pub mod args {
    use clap::Parser;

    #[derive(Parser, Debug, Clone)]
    pub struct RmsAction {
        #[clap(subcommand)]
        pub command: Cmd,

        #[clap(long, global = true, help = "URL of RMS API endpoint (required).")]
        pub url: Option<String>,

        #[clap(long, global = true, help = "Root CA path")]
        pub root_ca: Option<String>,

        #[clap(long, global = true, help = "Client certificate path")]
        pub client_cert: Option<String>,

        #[clap(long, global = true, help = "Client key path")]
        pub client_key: Option<String>,
    }

    #[derive(Parser, Debug, Clone)]
    #[clap(rename_all = "kebab_case")]
    pub enum Cmd {
        #[clap(about = "Get the full RMS inventory")]
        Inventory,
        #[clap(about = "Get the power on sequence")]
        PowerOnSequence(PowerOnSequence),
        #[clap(about = "Get the power state for a given node")]
        PowerState(PowerState),
        #[clap(about = "Get the firmware inventory for a given node")]
        FirmwareInventory(FirmwareInventory),
    }

    #[derive(Parser, Debug, Clone)]
    pub struct PowerOnSequence {
        #[clap(help = "Rack ID to get power sequence for")]
        pub rack_id: String,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct PowerState {
        #[clap(help = "Rack ID to get power sequence for")]
        pub rack_id: String,
        #[clap(help = "Node ID to get power state for")]
        pub node_id: String,
    }

    #[derive(Parser, Debug, Clone)]
    pub struct FirmwareInventory {
        #[clap(help = "Rack ID to get power sequence for")]
        pub rack_id: String,
        #[clap(help = "Node ID to get firmware inventory for")]
        pub node_id: String,
    }
}

pub mod cmds {
    use std::sync::Arc;

    use librms::RmsApi;

    use crate::rms::args::{FirmwareInventory, PowerOnSequence, PowerState};

    pub async fn get_all_inventory(rms_client: &Arc<dyn RmsApi>) -> eyre::Result<()> {
        let cmd = librms::protos::rack_manager::GetAllInventoryRequest::default();
        let response = rms_client.get_all_inventory(cmd).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }

    pub async fn power_on_sequence(
        args: &PowerOnSequence,
        rms_client: &Arc<dyn RmsApi>,
    ) -> eyre::Result<()> {
        let cmd = librms::protos::rack_manager::GetRackPowerOnSequenceRequest {
            metadata: None,
            rack_id: args.rack_id.clone(),
        };
        let response = rms_client.get_rack_power_on_sequence(cmd).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }

    pub async fn power_state(args: &PowerState, rms_client: &Arc<dyn RmsApi>) -> eyre::Result<()> {
        let cmd = librms::protos::rack_manager::GetPowerStateRequest {
            metadata: None,
            node_id: args.node_id.clone(),
            rack_id: args.rack_id.clone(),
        };
        let response = rms_client.get_power_state(cmd).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }

    pub async fn get_firmware_inventory(
        args: &FirmwareInventory,
        rms_client: &Arc<dyn RmsApi>,
    ) -> eyre::Result<()> {
        let cmd = librms::protos::rack_manager::GetNodeFirmwareInventoryRequest {
            metadata: None,
            node_id: args.node_id.clone(),
            rack_id: args.rack_id.clone(),
        };
        let response = rms_client.get_node_firmware_inventory(cmd).await?;
        println!("{}", serde_json::to_string_pretty(&response)?);
        Ok(())
    }
}
#[cfg(test)]
mod tests;

pub async fn action(action: RmsAction, config: &CliOptions) -> color_eyre::Result<()> {
    let url = if let Some(x) = action.url {
        x
    } else if let Some(y) = config.rms_api_url.clone() {
        y
    } else {
        eprintln!("No RMS API URL provided.");
        return Ok(());
    };
    let root_ca = if let Some(x) = action.root_ca {
        Some(x)
    } else {
        config.rms_root_ca_path.clone()
    };
    let client_cert = if let Some(x) = action.client_cert {
        Some(x)
    } else {
        config.rms_client_cert_path.clone()
    };
    let client_key = if let Some(x) = action.client_key {
        Some(x)
    } else {
        config.rms_client_key_path.clone()
    };
    let enforce_tls = !(root_ca.is_none() || client_cert.is_none() || client_key.is_none());

    // similar to libredfish
    let rms_client_config =
        librms::client_config::RmsClientConfig::new(root_ca, client_cert, client_key, enforce_tls);
    let rms_api_config = librms::client::RmsApiConfig::new(&url, &rms_client_config);
    let rms_client_pool = librms::RmsClientPool::new(&rms_api_config);
    let rms_client = rms_client_pool.create_client().await;

    match action.command {
        Cmd::Inventory => cmds::get_all_inventory(&rms_client).await,
        Cmd::PowerOnSequence(ref args) => cmds::power_on_sequence(args, &rms_client).await,
        Cmd::PowerState(ref args) => cmds::power_state(args, &rms_client).await,
        Cmd::FirmwareInventory(ref args) => cmds::get_firmware_inventory(args, &rms_client).await,
    }
}
