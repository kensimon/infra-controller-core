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

use crate::cfg::run::Run;
use crate::cfg::runtime::RuntimeContext;

pub mod args {
    use carbide_uuid::rack::RackId;
    use clap::{ArgGroup, Parser};
    use mac_address::MacAddress;
    use serde::{Deserialize, Serialize};

    #[derive(Parser, Debug, Serialize, Deserialize)]
    #[clap(group(ArgGroup::new("group").required(true).multiple(true).args(&[
    "bmc_username",
    "bmc_password",
    "shelf_serial_number",
    ])))]
    pub struct Args {
        #[clap(
            short = 'a',
            required = true,
            long,
            help = "BMC MAC Address of the expected power shelf"
        )]
        pub bmc_mac_address: MacAddress,
        #[clap(
            short = 'u',
            long,
            group = "group",
            requires("bmc_password"),
            help = "BMC username of the expected power shelf"
        )]
        pub bmc_username: Option<String>,
        #[clap(
            short = 'p',
            long,
            group = "group",
            requires("bmc_username"),
            help = "BMC password of the expected power shelf"
        )]
        pub bmc_password: Option<String>,
        #[clap(
            short = 's',
            long,
            group = "group",
            help = "Chassis serial number of the expected power shelf"
        )]
        pub shelf_serial_number: Option<String>,

        #[clap(
            long = "meta-name",
            value_name = "META_NAME",
            help = "The name that should be used as part of the Metadata for newly created Power Shelves. If empty, the Power Shelf Id will be used"
        )]
        pub meta_name: Option<String>,

        #[clap(
            long = "meta-description",
            value_name = "META_DESCRIPTION",
            help = "The description that should be used as part of the Metadata for newly created Power Shelves"
        )]
        pub meta_description: Option<String>,

        #[clap(
            long = "label",
            value_name = "LABEL",
            help = "A label that will be added as metadata for the newly created Machine. The labels key and value must be separated by a : character",
            action = clap::ArgAction::Append
        )]
        pub labels: Option<Vec<String>>,

        #[clap(
            long = "host_name",
            value_name = "HOST_NAME",
            help = "Host name of the power shelf",
            action = clap::ArgAction::Append
        )]
        pub host_name: Option<String>,

        #[clap(
            long = "rack_id",
            value_name = "RACK_ID",
            help = "Rack ID for this power shelf",
            action = clap::ArgAction::Append
        )]
        pub rack_id: Option<RackId>,

        #[clap(
            long = "ip_address",
            value_name = "IP_ADDRESS",
            help = "IP address of the power shelf",
            action = clap::ArgAction::Append
        )]
        pub ip_address: Option<String>,
    }

    impl Args {
        pub fn validate(&self) -> Result<(), String> {
            // TODO: It is possible to do these checks by clap itself, via arg groups
            if self.bmc_username.is_none()
                && self.bmc_password.is_none()
                && self.shelf_serial_number.is_none()
            {
                return Err("One of the following options must be specified: bmc-user-name and bmc-password or shelf-serial-number".to_string());
            }
            Ok(())
        }
    }
}

pub mod cmd {
    use super::args::Args;
    use crate::metadata::parse_rpc_labels;
    use crate::rpc::ApiClient;

    pub async fn update(data: Args, api_client: &ApiClient) -> color_eyre::Result<()> {
        if let Err(e) = data.validate() {
            eprintln!("{e}");
            return Ok(());
        }
        let metadata = rpc::forge::Metadata {
            name: data.meta_name.unwrap_or_default(),
            description: data.meta_description.unwrap_or_default(),
            labels: parse_rpc_labels(data.labels.unwrap_or_default()),
        };
        api_client
            .update_expected_power_shelf(
                data.bmc_mac_address,
                data.bmc_username,
                data.bmc_password,
                data.shelf_serial_number,
                data.rack_id,
                data.ip_address,
                metadata,
            )
            .await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::update(self, &ctx.api_client).await?;
        Ok(())
    }
}
