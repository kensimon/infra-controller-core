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
    pub enum Args {
        #[clap(about = "Show tests")]
        Show(ShowTestOptions),
        #[clap(about = "Verify a given test")]
        Verify(VerifyTestOptions),
        #[clap(about = "Add new test case")]
        Add(AddTestOptions),
        #[clap(about = "Update existing test case")]
        Update(UpdateTestOptions),
        #[clap(about = "Enabled a test")]
        Enable(EnableDisableTestOptions),
        #[clap(about = "Disable a test")]
        Disable(EnableDisableTestOptions),
    }

    #[derive(Parser, Debug)]
    pub struct ShowTestOptions {
        #[clap(short, long, help = "Unique identification of the test")]
        pub test_id: Option<String>,

        #[clap(short, long, help = "List of platforms")]
        pub platforms: Vec<String>,

        #[clap(short, long, help = "List of contexts/tags")]
        pub contexts: Vec<String>,

        #[clap(long, default_value = "false", help = "List unverfied tests also.")]
        pub show_un_verfied: bool,
    }

    #[derive(Parser, Debug)]
    pub struct VerifyTestOptions {
        #[clap(short, long, help = "Unique identification of the test")]
        pub test_id: String,

        #[clap(short, long, help = "Version to be verify")]
        pub version: String,
    }

    #[derive(Parser, Debug)]
    pub struct EnableDisableTestOptions {
        #[clap(short, long, help = "Unique identification of the test")]
        pub test_id: String,

        #[clap(short, long, help = "Version to be verify")]
        pub version: String,
    }

    #[derive(Parser, Debug)]
    pub struct UpdateTestOptions {
        #[clap(long, help = "Unique identification of the test")]
        pub test_id: String,

        #[clap(long, help = "Version to be verify")]
        pub version: String,

        #[clap(long, help = "List of contexts")]
        pub contexts: Vec<String>,

        #[clap(long, help = "Container image name")]
        pub img_name: Option<String>,

        #[clap(long, help = "Run command using chroot in case of container")]
        pub execute_in_host: Option<bool>,

        #[clap(long, help = "Container args", allow_hyphen_values = true)]
        pub container_arg: Option<String>,

        #[clap(long, help = "Description")]
        pub description: Option<String>,

        #[clap(long, help = "Command ")]
        pub command: Option<String>,

        #[clap(long, help = "Command args", allow_hyphen_values = true)]
        pub args: Option<String>,

        #[clap(long, help = "Command output error file ")]
        pub extra_err_file: Option<String>,

        #[clap(long, help = "Command output file ")]
        pub extra_output_file: Option<String>,

        #[clap(long, help = "External file")]
        pub external_config_file: Option<String>,

        #[clap(long, help = "Pre condition")]
        pub pre_condition: Option<String>,

        #[clap(long, help = "Command Timeout")]
        pub timeout: Option<i64>,

        #[clap(long, help = "List of supported platforms")]
        pub supported_platforms: Vec<String>,

        #[clap(long, help = "List of custom tags")]
        pub custom_tags: Vec<String>,

        #[clap(long, help = "List of system components")]
        pub components: Vec<String>,

        #[clap(long, help = "Enable the test")]
        pub is_enabled: Option<bool>,
    }

    #[derive(Parser, Debug)]
    pub struct AddTestOptions {
        #[clap(long, help = "Name of the test case")]
        pub name: String,

        #[clap(long, help = "Command of the test case")]
        pub command: String,

        #[clap(long, help = "Command args", allow_hyphen_values = true)]
        pub args: String,

        #[clap(long, help = "List of contexts")]
        pub contexts: Vec<String>,

        #[clap(long, help = "Container image name")]
        pub img_name: Option<String>,

        #[clap(long, help = "Run command using chroot in case of container")]
        pub execute_in_host: Option<bool>,

        #[clap(long, help = "Container args", allow_hyphen_values = true)]
        pub container_arg: Option<String>,

        #[clap(long, help = "Description")]
        pub description: Option<String>,

        #[clap(long, help = "Command output error file ")]
        pub extra_err_file: Option<String>,

        #[clap(long, help = "Command output file ")]
        pub extra_output_file: Option<String>,

        #[clap(long, help = "External file")]
        pub external_config_file: Option<String>,

        #[clap(long, help = "Pre condition")]
        pub pre_condition: Option<String>,

        #[clap(long, help = "Command Timeout")]
        pub timeout: Option<i64>,

        #[clap(long, help = "List of supported platforms")]
        pub supported_platforms: Vec<String>,

        #[clap(long, help = "List of custom tags")]
        pub custom_tags: Vec<String>,

        #[clap(long, help = "List of system components")]
        pub components: Vec<String>,

        #[clap(long, help = "Enable the test")]
        pub is_enabled: Option<bool>,

        #[clap(long, help = "Is read-only")]
        pub read_only: Option<bool>,
    }
}

pub mod cmd {
    use std::fmt::Write;

    use ::rpc::admin_cli::OutputFormat;
    use ::rpc::forge::{
        MachineValidationTestEnableDisableTestRequest, MachineValidationTestUpdateRequest,
        MachineValidationTestVerfiedRequest, {self as forgerpc},
    };
    use prettytable::{Table, row};

    use super::args::{
        AddTestOptions, EnableDisableTestOptions, ShowTestOptions, UpdateTestOptions,
        VerifyTestOptions,
    };
    use super::*;
    use crate::rpc::ApiClient;

    pub async fn show_tests(
        api_client: &ApiClient,
        args: ShowTestOptions,
        output_format: OutputFormat,
        extended: bool,
    ) -> CarbideCliResult<()> {
        let tests = api_client
            .get_machine_validation_tests(
                args.test_id,
                args.platforms,
                args.contexts,
                args.show_un_verfied,
            )
            .await?;
        if extended {
            let _ = show_tests_details(output_format == OutputFormat::Json, tests);
        } else {
            convert_tests_to_nice_table(tests.tests).printstd();
        }

        Ok(())
    }

    fn show_tests_details(
        is_json: bool,
        test: forgerpc::MachineValidationTestsGetResponse,
    ) -> CarbideCliResult<()> {
        if is_json {
            for test in test.tests {
                println!("{}", serde_json::to_string_pretty(&test)?);
            }
        } else {
            println!(
                "{}",
                convert_tests_to_nice_format(test.tests).unwrap_or_else(|x| x.to_string())
            );
        }
        Ok(())
    }

    fn convert_tests_to_nice_table(tests: Vec<forgerpc::MachineValidationTest>) -> Box<Table> {
        let mut table = Table::new();

        table.set_titles(row![
            "TestId",
            "Name",
            "Command",
            "Timeout",
            "IsVerified",
            "Version",
            "IsEnabled",
        ]);

        for test in tests {
            table.add_row(row![
                test.test_id,
                test.name,
                test.command,
                test.timeout.unwrap_or_default().to_string(),
                test.verified,
                test.version,
                test.is_enabled,
            ]);
        }

        table.into()
    }

    fn convert_tests_to_nice_format(
        tests: Vec<forgerpc::MachineValidationTest>,
    ) -> CarbideCliResult<String> {
        let width = 14;
        let mut lines = String::new();
        if tests.is_empty() {
            return Ok(lines);
        }
        // data.clear();
        for test in tests {
            writeln!(
                &mut lines,
                "\t------------------------------------------------------------------------"
            )?;
            let contexts = match serde_json::to_string(&test.contexts) {
                Ok(msg) => msg,
                Err(_) => "[]".to_string(),
            };
            let platforms = match serde_json::to_string(&test.supported_platforms) {
                Ok(msg) => msg,
                Err(_) => "[]".to_string(),
            };
            let custom_tags = match serde_json::to_string(&test.custom_tags) {
                Ok(msg) => msg,
                Err(_) => "[]".to_string(),
            };
            let components = match serde_json::to_string(&test.components) {
                Ok(msg) => msg,
                Err(_) => "[]".to_string(),
            };

            let details = vec![
                ("TestId", test.test_id),
                ("Name", test.name),
                ("Description", test.description.unwrap_or_default()),
                ("Command", test.command),
                ("Args", test.args),
                ("Contexts", contexts),
                ("PreCondition", test.pre_condition.unwrap_or_default()),
                ("TimeOut", test.timeout.unwrap().to_string()),
                ("CustomTags", custom_tags),
                ("Components", components),
                ("SupportedPlatforms", platforms),
                ("ImageName", test.img_name.unwrap_or_default()),
                ("ContainerArgs", test.container_arg.unwrap_or_default()),
                (
                    "ExecuteInHost",
                    test.execute_in_host.unwrap_or_default().to_string(),
                ),
                ("ExtraErrorFile", test.extra_err_file.unwrap_or_default()),
                (
                    "ExtraOutPutFile",
                    test.extra_output_file.unwrap_or_default(),
                ),
                (
                    "ExternalConfigFile",
                    test.external_config_file.unwrap_or_default(),
                ),
                ("Version", test.version.to_string()),
                ("LastModifiedAt", test.last_modified_at),
                ("LastModifiedBy", test.modified_by),
                ("IsVerified", test.verified.to_string()),
                ("IsReadOnly", test.read_only.to_string()),
                ("IsEnabled", test.is_enabled.to_string()),
            ];

            for (key, value) in details {
                writeln!(&mut lines, "{key:<width$}: {value}")?;
            }
            writeln!(
                &mut lines,
                "\t------------------------------------------------------------------------"
            )?;
        }
        Ok(lines)
    }

    pub async fn machine_validation_test_verfied(
        api_client: &ApiClient,
        options: VerifyTestOptions,
    ) -> CarbideCliResult<()> {
        api_client
            .0
            .machine_validation_test_verfied(MachineValidationTestVerfiedRequest {
                test_id: options.test_id,
                version: options.version,
            })
            .await?;
        Ok(())
    }

    pub async fn machine_validation_test_enable(
        api_client: &ApiClient,
        options: EnableDisableTestOptions,
    ) -> CarbideCliResult<()> {
        api_client
            .0
            .machine_validation_test_enable_disable_test(
                MachineValidationTestEnableDisableTestRequest {
                    test_id: options.test_id,
                    version: options.version,
                    is_enabled: true,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn machine_validation_test_disable(
        api_client: &ApiClient,
        options: EnableDisableTestOptions,
    ) -> CarbideCliResult<()> {
        api_client
            .0
            .machine_validation_test_enable_disable_test(
                MachineValidationTestEnableDisableTestRequest {
                    test_id: options.test_id,
                    version: options.version,
                    is_enabled: false,
                },
            )
            .await?;
        Ok(())
    }

    pub async fn machine_validation_test_update(
        api_client: &ApiClient,
        options: UpdateTestOptions,
    ) -> CarbideCliResult<()> {
        let payload = forgerpc::machine_validation_test_update_request::Payload {
            contexts: options.contexts,
            img_name: options.img_name,
            execute_in_host: options.execute_in_host,
            container_arg: options.container_arg,
            command: options.command,
            args: options.args,
            extra_err_file: options.extra_err_file,
            external_config_file: options.external_config_file,
            pre_condition: options.pre_condition,
            timeout: options.timeout,
            extra_output_file: options.extra_output_file,
            supported_platforms: options.supported_platforms,
            custom_tags: options.custom_tags,
            components: options.components,
            is_enabled: options.is_enabled,
            description: options.description,
            verified: None,
            name: None,
        };
        api_client
            .0
            .update_machine_validation_test(MachineValidationTestUpdateRequest {
                test_id: options.test_id,
                version: options.version,
                payload: Some(payload),
            })
            .await?;
        Ok(())
    }

    pub async fn machine_validation_test_add(
        api_client: &ApiClient,
        options: AddTestOptions,
    ) -> CarbideCliResult<()> {
        let mut contexts = vec!["OnDemand".to_string()];
        if !options.contexts.is_empty() {
            contexts = options.contexts;
        }

        let mut supported_platforms = vec!["New_Sku".to_string()];
        if !options.supported_platforms.is_empty() {
            supported_platforms = options.supported_platforms;
        }
        let mut description = Some("new test case".to_string());
        if options.description.is_some() {
            description = options.description;
        }
        let request = forgerpc::MachineValidationTestAddRequest {
            name: options.name,
            description,
            contexts,
            img_name: options.img_name,
            execute_in_host: options.execute_in_host,
            container_arg: options.container_arg,
            command: options.command,
            args: options.args,
            extra_err_file: options.extra_err_file,
            external_config_file: options.external_config_file,
            pre_condition: options.pre_condition,
            timeout: options.timeout,
            extra_output_file: options.extra_output_file,
            supported_platforms,
            read_only: options.read_only,
            custom_tags: options.custom_tags,
            components: options.components,
            is_enabled: options.is_enabled,
        };
        api_client.0.add_machine_validation_test(request).await?;
        Ok(())
    }
}

impl Run for Args {
    async fn run(self, ctx: &mut RuntimeContext) -> CarbideCliResult<()> {
        match self {
            Args::Show(options) => {
                cmd::show_tests(
                    &ctx.api_client,
                    options,
                    ctx.config.format,
                    ctx.config.extended,
                )
                .await
            }
            Args::Verify(options) => {
                cmd::machine_validation_test_verfied(&ctx.api_client, options).await
            }
            Args::Enable(options) => {
                cmd::machine_validation_test_enable(&ctx.api_client, options).await
            }
            Args::Disable(options) => {
                cmd::machine_validation_test_disable(&ctx.api_client, options).await
            }
            Args::Add(options) => cmd::machine_validation_test_add(&ctx.api_client, options).await,
            Args::Update(options) => {
                cmd::machine_validation_test_update(&ctx.api_client, options).await
            }
        }
    }
}
