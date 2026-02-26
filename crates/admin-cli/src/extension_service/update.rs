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
    use clap::Parser;

    #[derive(Parser, Debug)]
    pub struct Args {
        #[clap(short = 'i', long = "id", help = "The extension service ID to update")]
        pub service_id: String,

        #[clap(
            short = 'n',
            long = "name",
            help = "New extension service name (optional)"
        )]
        pub service_name: Option<String>,

        #[clap(long, help = "New extension service description (optional)")]
        pub description: Option<String>,

        #[clap(short = 'd', long, help = "New extension service data")]
        pub data: String,

        #[clap(long, help = "New registry URL for the service credential (optional)")]
        pub registry_url: Option<String>,

        #[clap(
            short = 'u',
            long,
            help = "New username for the service credential (optional)"
        )]
        pub username: Option<String>,

        #[clap(
            short = 'p',
            long,
            help = "New password for the service credential (optional)"
        )]
        pub password: Option<String>,

        #[clap(
            long,
            help = "Update only if current number of versions matches this number (optional)"
        )]
        pub if_version_ctr_match: Option<i32>,

        #[clap(
            long,
            help = "JSON array containing a defined set of extension observability configs (optional)"
        )]
        pub observability: Option<String>,
    }
}

pub mod cmd {
    use ::rpc::admin_cli::CarbideCliError;
    use ::rpc::admin_cli::output::OutputFormat;
    use ::rpc::forge::dpu_extension_service_credential::Type;

    use super::args::Args;
    use super::*;
    use crate::extension_service::show::cmd::convert_extension_services_to_table;
    use crate::rpc::ApiClient;

    pub async fn handle_update(
        args: Args,
        output_format: OutputFormat,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        let is_json = output_format == OutputFormat::Json;

        let credential =
            if args.username.is_some() || args.password.is_some() || args.registry_url.is_some() {
                if args.username.is_none() || args.password.is_none() || args.registry_url.is_none()
                {
                    return Err(CarbideCliError::GenericError(
                    "All of username, password and registry URL are required to create credential"
                        .to_string(),
                ));
                }

                Some(::rpc::forge::DpuExtensionServiceCredential {
                    registry_url: args.registry_url.unwrap(),
                    r#type: Some(Type::UsernamePassword(rpc::forge::UsernamePassword {
                        username: args.username.unwrap(),
                        password: args.password.unwrap(),
                    })),
                })
            } else {
                None
            };

        let observability = if let Some(r) = args.observability {
            serde_json::from_str(&r)?
        } else {
            vec![]
        };

        let extension_service = api_client
            .update_extension_service(
                args.service_id,
                args.service_name,
                args.description,
                args.data,
                credential,
                observability,
                args.if_version_ctr_match,
            )
            .await?;

        if is_json {
            println!("{}", serde_json::to_string_pretty(&extension_service)?);
        } else {
            convert_extension_services_to_table(&[extension_service]).printstd();
        }

        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        cmd::handle_update(self, ctx.config.format, &ctx.api_client).await
    }
}
