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

    use crate::extension_service::common::ExtensionServiceType;

    #[derive(Parser, Debug, Clone)]
    pub struct Args {
        #[clap(
            short = 'i',
            long = "id",
            help = "The extension service ID to create (optional)"
        )]
        pub service_id: Option<String>,

        #[clap(short = 'n', long = "name", help = "Extension service name")]
        pub service_name: String,

        #[clap(short = 't', long = "type", help = "Extension service type")]
        pub service_type: ExtensionServiceType,

        #[clap(long, help = "Extension service description (optional)")]
        pub description: Option<String>,

        #[clap(long, help = "Tenant organization ID")]
        pub tenant_organization_id: Option<String>,

        #[clap(short = 'd', long, help = "Extension service data")]
        pub data: String,

        #[clap(long, help = "Registry URL for the service credential (optional)")]
        pub registry_url: Option<String>,

        #[clap(long, help = "Username for the service credential (optional)")]
        pub username: Option<String>,

        #[clap(long, help = "Password for the service credential (optional)")]
        pub password: Option<String>,

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

    pub async fn handle_create(
        args: Args,
        output_format: OutputFormat,
        api_client: &ApiClient,
    ) -> CarbideCliResult<()> {
        let is_json = output_format == OutputFormat::Json;

        let credential =
            if args.username.is_some() || args.password.is_some() || args.registry_url.is_some() {
                // This check is for KubernetesPod service credentials, must be modified if we add more service types
                if args.username.is_none() || args.password.is_none() || args.registry_url.is_none()
                {
                    return Err(CarbideCliError::GenericError(
                    "All of username, password and registry URL are required to create credential"
                        .to_string(),
                ));
                }

                Some(::rpc::forge::DpuExtensionServiceCredential {
                    registry_url: args.registry_url.unwrap_or_default(),
                    r#type: Some(Type::UsernamePassword(rpc::forge::UsernamePassword {
                        username: args.username.unwrap_or_default(),
                        password: args.password.unwrap_or_default(),
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
            .create_extension_service(
                args.service_id,
                args.service_name,
                args.tenant_organization_id.unwrap_or_default(),
                args.service_type as i32,
                args.description,
                args.data,
                credential,
                observability,
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
        cmd::handle_create(self, ctx.config.format, &ctx.api_client).await
    }
}
