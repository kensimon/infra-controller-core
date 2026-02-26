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

use ::rpc::admin_cli::CarbideCliResult;
pub use args::Args;
use carbide_uuid::machine::MachineId;

use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use clap::Parser;

    use super::*;

    #[derive(Parser, Debug)]
    pub enum Args {
        #[clap(about = "Show the hardware info of the machine")]
        Show(ShowMachineHardwareInfo),
        #[clap(subcommand, about = "Update the hardware info of the machine")]
        Update(MachineHardwareInfo),
    }

    #[derive(Parser, Debug)]
    pub struct ShowMachineHardwareInfo {
        #[clap(long, help = "Show the hardware info of this Machine ID")]
        pub machine: MachineId,
    }

    #[derive(Parser, Debug)]
    pub enum MachineHardwareInfo {
        //Cpu(MachineTopologyCommandCpu),
        #[clap(about = "Update the GPUs of this machine")]
        Gpus(MachineHardwareInfoGpus),
        //Memory(MachineTopologyCommandMemory),
        //Storage(MachineTopologyCommandStorage),
        //Network(MachineTopologyCommandNetwork),
        //Infiniband(MachineTopologyCommandInfiniband),
        //Dpu(MachineTopologyCommandDpu),
    }

    #[derive(Parser, Debug)]
    pub struct MachineHardwareInfoGpus {
        #[clap(long, help = "Machine ID of the server containing the GPUs")]
        pub machine: MachineId,
        #[clap(
            long,
            help = "JSON file containing GPU info. It should contain an array of JSON objects like this:
            {
                \"name\": \"string\",
                \"serial\": \"string\",
                \"driver_version\": \"string\",
                \"vbios_version\": \"string\",
                \"inforom_version\": \"string\",
                \"total_memory\": \"string\",
                \"frequency\": \"string\",
                \"pci_bus_id\": \"string\"
            }
            Pass an empty array if you want to remove GPUs."
        )]
        pub gpu_json_file: std::path::PathBuf,
    }
}

pub mod cmd {
    use std::fs;
    use std::pin::Pin;

    use ::rpc::admin_cli::{CarbideCliError, OutputFormat};
    use ::rpc::forge as forgerpc;

    use super::args::MachineHardwareInfoGpus;
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn handle_update_machine_hardware_info_gpus(
        api_client: &ApiClient,
        gpus: MachineHardwareInfoGpus,
    ) -> CarbideCliResult<()> {
        let gpu_file_contents = fs::read_to_string(gpus.gpu_json_file)?;
        let gpus_from_json: Vec<::rpc::machine_discovery::Gpu> =
            serde_json::from_str(&gpu_file_contents)?;
        api_client
            .update_machine_hardware_info(
                gpus.machine,
                forgerpc::MachineHardwareInfoUpdateType::Gpus,
                gpus_from_json,
            )
            .await
    }

    pub fn handle_show_machine_hardware_info(
        _api_client: &ApiClient,
        _output_file: &mut Pin<Box<dyn tokio::io::AsyncWrite>>,
        _output_format: &OutputFormat,
        _machine_id: MachineId,
    ) -> CarbideCliResult<()> {
        Err(CarbideCliError::NotImplemented(
            "machine hardware output".to_string(),
        ))
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        match self {
            Args::Show(show_cmd) => {
                cmd::handle_show_machine_hardware_info(
                    &ctx.api_client,
                    &mut ctx.output_file,
                    &ctx.config.format,
                    show_cmd.machine,
                )?;
            }
            Args::Update(capability) => match capability {
                args::MachineHardwareInfo::Gpus(gpus) => {
                    cmd::handle_update_machine_hardware_info_gpus(&ctx.api_client, gpus).await?;
                }
            },
        }
        Ok(())
    }
}
