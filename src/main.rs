#![warn(clippy::pedantic)]
#![warn(clippy::style)]
#![warn(clippy::arithmetic_side_effects)]
#![warn(clippy::as_underscore)]
#![warn(clippy::assertions_on_result_states)]
#![warn(clippy::branches_sharing_code)]
#![warn(clippy::cargo_common_metadata)]
#![warn(clippy::clear_with_drain)]
#![warn(clippy::clone_on_ref_ptr)]
// #![warn(clippy::cognitive_complexity)] // later
#![warn(clippy::collection_is_never_read)]
#![warn(clippy::create_dir)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::debug_assert_with_mut_call)]
#![warn(clippy::decimal_literal_representation)]
#![warn(clippy::default_union_representation)]
#![warn(clippy::deref_by_slicing)]
#![warn(clippy::derive_partial_eq_without_eq)]
#![warn(clippy::empty_drop)]
#![warn(clippy::empty_line_after_doc_comments)]
#![warn(clippy::empty_line_after_outer_attr)]
#![warn(clippy::empty_structs_with_brackets)]
#![warn(clippy::equatable_if_let)]
#![warn(clippy::fallible_impl_from)]
#![warn(clippy::filetype_is_file)]
#![warn(clippy::float_cmp_const)]
#![warn(clippy::fn_to_numeric_cast_any)]
#![warn(clippy::format_push_string)]
#![warn(clippy::get_unwrap)]
#![warn(clippy::if_then_some_else_none)]
#![warn(clippy::impl_trait_in_params)]
#![warn(clippy::imprecise_flops)]
#![warn(clippy::iter_on_empty_collections)]
#![warn(clippy::iter_on_single_items)]
#![warn(clippy::iter_with_drain)]
#![warn(clippy::large_stack_frames)]
#![warn(clippy::let_underscore_untyped)]
#![warn(clippy::lossy_float_literal)]
#![warn(clippy::manual_clamp)]
#![warn(clippy::mem_forget)]
#![warn(clippy::min_ident_chars)]
#![warn(clippy::mixed_read_write_in_expression)]
#![warn(clippy::multiple_inherent_impl)]
#![warn(clippy::needless_collect)]
#![warn(clippy::needless_pass_by_ref_mut)]
#![warn(clippy::negative_feature_names)]
#![warn(clippy::nonstandard_macro_braces)]
#![warn(clippy::or_fun_call)]
#![warn(clippy::path_buf_push_overwrite)]
#![warn(clippy::pub_without_shorthand)]
#![warn(clippy::rc_buffer)]
#![warn(clippy::rc_mutex)]
#![warn(clippy::readonly_write_lock)]
#![warn(clippy::redundant_pub_crate)]
#![warn(clippy::redundant_clone)]
#![warn(clippy::rest_pat_in_fully_bound_structs)]
#![warn(clippy::same_name_method)]
#![warn(clippy::self_named_module_files)]
#![warn(clippy::semicolon_inside_block)]
#![warn(clippy::significant_drop_in_scrutinee)]
#![warn(clippy::significant_drop_tightening)]
#![warn(clippy::str_to_string)]
#![warn(clippy::string_lit_chars_any)]
#![warn(clippy::string_to_string)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::suspicious_operation_groupings)]
#![warn(clippy::suspicious_xor_used_as_pow)]
#![warn(clippy::tests_outside_test_module)]
#![warn(clippy::trait_duplication_in_bounds)]
#![warn(clippy::trivial_regex)]
#![warn(clippy::try_err)]
#![warn(clippy::type_repetition_in_bounds)]
#![warn(clippy::unnecessary_struct_initialization)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(clippy::unseparated_literal_suffix)]
#![warn(clippy::unused_peekable)]
#![warn(clippy::unused_rounding)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::useless_let_if_seq)]
#![warn(clippy::verbose_file_reads)]
#![warn(clippy::wildcard_dependencies)]
#![warn(absolute_paths_not_starting_with_crate)]
#![warn(keyword_idents)]
#![warn(let_underscore_drop)]
#![warn(macro_use_extern_crate)]
#![warn(meta_variable_misuse)]
#![warn(missing_abi)]
#![warn(unsafe_op_in_unsafe_fn)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::too_many_lines)] // warn later with cognitive_complexity
#![allow(uncommon_codepoints)]

use clap::{Parser, Subcommand};

pub mod client;
pub mod common;
pub mod datagram;
pub mod message;
pub mod server;
pub mod types;
pub mod version;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List the files a namida server has available for download.
    Dir(client::dir::Parameter),

    /// Download one or more files from a namida server.
    Get(client::get::Parameter),

    /// Start a namida server process, serving the specified files.
    Serve(server::Parameter),
}

#[allow(clippy::missing_errors_doc)]
pub fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Get(parameter) => {
            client::get::run(parameter)?;
        }
        Commands::Dir(parameter) => {
            client::dir::run(parameter)?;
        }
        Commands::Serve(parameter) => {
            server::main::serve(parameter)?;
        }
    }

    Ok(())
}
