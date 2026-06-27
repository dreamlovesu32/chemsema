use chemcore_desktop_service::DesktopDocumentService;
use chemcore_engine::Engine;
use serde_json::Map;
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

const HELP: &str = r#"ChemCore CLI

Usage:
  chemcore-cli inspect <input> [--include summary,objects,molecules,resources,styles] [--out <path>] [--pretty]
  chemcore-cli new [commands.json|-] --out <path> [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]
  chemcore-cli convert <input> <output> [--format <format>]
  chemcore-cli export <input> <output> [--format <format>]
  chemcore-cli run <input> <commands.json|-> [--out <path>] [--save-format <format>] [--results <path>] [--document-json <path>] [--inspect-after <include|none>] [--pretty] [--quiet]

Formats:
  json, ccjs, ccjz, cdxml, cdx, sdf, svg

Examples:
  chemcore-cli inspect figure1.cdxml --pretty
  chemcore-cli new --out blank.ccjs
  chemcore-cli new commands.json --out generated.cdxml --results results.json --pretty
  chemcore-cli convert figure1.cdxml figure1.svg
  chemcore-cli run figure1.cdxml commands.json --out edited.cdxml --results results.json --pretty
"#;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = args.first().map(String::as_str) else {
        print!("{HELP}");
        return Ok(());
    };
    if matches!(command, "-h" | "--help" | "help") {
        print!("{HELP}");
        return Ok(());
    }

    match command {
        "inspect" => inspect_command(&args[1..]),
        "new" => new_command(&args[1..]),
        "convert" => convert_command(&args[1..]),
        "export" => convert_command(&args[1..]),
        "run" => run_command_script(&args[1..]),
        other => Err(format!(
            "Unknown command '{other}'. Run chemcore-cli --help."
        )),
    }
}

fn new_command(args: &[String]) -> Result<(), String> {
    let mut script = None;
    let mut output = None;
    let mut save_format = None;
    let mut results = None;
    let mut document_json_output = None;
    let mut inspect_after = default_inspect_after();
    let mut pretty = false;
    let mut quiet = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--save-format" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--save-format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--results" => {
                index += 1;
                results = Some(
                    args.get(index)
                        .ok_or_else(|| "--results requires a path.".to_string())?
                        .clone(),
                );
            }
            "--document-json" => {
                index += 1;
                document_json_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--document-json requires a path.".to_string())?
                        .clone(),
                );
            }
            "--inspect-after" => {
                index += 1;
                inspect_after = parse_inspect_after_value(
                    args.get(index)
                        .ok_or_else(|| "--inspect-after requires a value.".to_string())?,
                );
            }
            "--no-inspect-after" => inspect_after = None,
            "--pretty" => pretty = true,
            "--quiet" => quiet = true,
            value if script.is_none() => script = Some(value.to_string()),
            value => return Err(format!("Unexpected new argument '{value}'.")),
        }
        index += 1;
    }

    let output = output.ok_or_else(|| "new requires --out <path>.".to_string())?;
    if document_json_output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --document-json is '-'.".to_string());
    }
    let mut engine = Engine::new();
    let mut execution = if let Some(script) = script.as_deref() {
        execute_command_file(&mut engine, script, inspect_after.as_deref())
    } else {
        empty_script_execution(&mut engine, inspect_after.as_deref())
    };
    write_optional_document_json(
        &mut execution,
        &engine,
        document_json_output.as_deref(),
        "documentJson",
    );
    if execution.ok {
        match write_engine_output(&engine, &output, save_format.as_deref()) {
            Ok(()) => set_report_field(
                &mut execution.report,
                "save",
                json!({
                    "ok": true,
                    "path": output,
                    "format": save_format
                        .as_deref()
                        .map(str::to_string)
                        .or_else(|| infer_format_from_path(&output)),
                }),
            ),
            Err(error) => {
                execution.ok = false;
                execution.error_message = Some(error.clone());
                set_report_field(
                    &mut execution.report,
                    "save",
                    json!({
                        "ok": false,
                        "path": output,
                        "error": {
                            "stage": "save-output",
                            "message": error,
                        }
                    }),
                );
            }
        }
    } else {
        let reason = execution
            .error_message
            .clone()
            .unwrap_or_else(|| "command script failed".to_string());
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": false,
                "path": output,
                "skipped": true,
                "reason": reason,
            }),
        );
    }
    set_report_field(&mut execution.report, "ok", json!(execution.ok));
    if !quiet {
        write_json_value(execution.report.clone(), results.as_deref(), pretty)?;
    }
    if !execution.ok {
        return Err(execution
            .error_message
            .unwrap_or_else(|| "Command script failed.".to_string()));
    }
    Ok(())
}

fn inspect_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut include = Vec::new();
    let mut output = None;
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--include" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| "--include requires a value.".to_string())?;
                include = split_csv(value);
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected inspect argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "inspect requires an input file.".to_string())?;
    let mut engine = load_engine_from_file(&input)?;
    let mut command = json!({ "type": "inspect-document" });
    if !include.is_empty() {
        command["include"] = json!(include);
    }
    let result = execute_json_command(&mut engine, command)?;
    write_json_value(
        result
            .get("output")
            .cloned()
            .ok_or_else(|| "inspect command did not return output.".to_string())?,
        output.as_deref(),
        pretty,
    )
}

fn convert_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut format = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--format" | "-f" => {
                index += 1;
                format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            value if input.is_none() => input = Some(value.to_string()),
            value if output.is_none() => output = Some(value.to_string()),
            value => return Err(format!("Unexpected convert/export argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "convert/export requires an input file.".to_string())?;
    let output = output.ok_or_else(|| "convert/export requires an output path.".to_string())?;
    let engine = load_engine_from_file(&input)?;
    write_engine_output(&engine, &output, format.as_deref())
}

fn run_command_script(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut script = None;
    let mut output = None;
    let mut save_format = None;
    let mut results = None;
    let mut document_json_output = None;
    let mut inspect_after = default_inspect_after();
    let mut pretty = false;
    let mut quiet = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--save-format" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--save-format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                save_format = Some(
                    args.get(index)
                        .ok_or_else(|| "--format requires a value.".to_string())?
                        .clone(),
                );
            }
            "--results" => {
                index += 1;
                results = Some(
                    args.get(index)
                        .ok_or_else(|| "--results requires a path.".to_string())?
                        .clone(),
                );
            }
            "--document-json" => {
                index += 1;
                document_json_output = Some(
                    args.get(index)
                        .ok_or_else(|| "--document-json requires a path.".to_string())?
                        .clone(),
                );
            }
            "--inspect-after" => {
                index += 1;
                inspect_after = parse_inspect_after_value(
                    args.get(index)
                        .ok_or_else(|| "--inspect-after requires a value.".to_string())?,
                );
            }
            "--no-inspect-after" => inspect_after = None,
            "--pretty" => pretty = true,
            "--quiet" => quiet = true,
            value if input.is_none() => input = Some(value.to_string()),
            value if script.is_none() => script = Some(value.to_string()),
            value => return Err(format!("Unexpected run argument '{value}'.")),
        }
        index += 1;
    }

    let input = input.ok_or_else(|| "run requires an input file.".to_string())?;
    let script = script.ok_or_else(|| "run requires a command JSON file.".to_string())?;
    if output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --out is '-'.".to_string());
    }
    if document_json_output.as_deref() == Some("-") && !quiet && results.is_none() {
        return Err("Use --results or --quiet when --document-json is '-'.".to_string());
    }

    let mut engine = load_engine_from_file(&input)?;
    let mut execution = execute_command_file(&mut engine, &script, inspect_after.as_deref());
    write_optional_document_json(
        &mut execution,
        &engine,
        document_json_output.as_deref(),
        "documentJson",
    );

    if execution.ok {
        if let Some(output) = output.as_deref() {
            match write_engine_output(&engine, output, save_format.as_deref()) {
                Ok(()) => set_report_field(
                    &mut execution.report,
                    "save",
                    json!({
                        "ok": true,
                        "path": output,
                        "format": save_format
                            .as_deref()
                            .map(str::to_string)
                            .or_else(|| infer_format_from_path(output)),
                    }),
                ),
                Err(error) => {
                    execution.ok = false;
                    execution.error_message = Some(error.clone());
                    set_report_field(
                        &mut execution.report,
                        "save",
                        json!({
                            "ok": false,
                            "path": output,
                            "error": {
                                "stage": "save-output",
                                "message": error,
                            }
                        }),
                    );
                }
            }
        } else {
            set_report_field(
                &mut execution.report,
                "save",
                json!({
                    "ok": true,
                    "skipped": true,
                    "reason": "--out was not provided",
                }),
            );
        }
    } else {
        let reason = execution
            .error_message
            .clone()
            .unwrap_or_else(|| "command script failed".to_string());
        set_report_field(
            &mut execution.report,
            "save",
            json!({
                "ok": false,
                "skipped": true,
                "reason": reason,
            }),
        );
    }
    set_report_field(&mut execution.report, "ok", json!(execution.ok));
    if !quiet {
        write_json_value(execution.report.clone(), results.as_deref(), pretty)?;
    }
    if !execution.ok {
        return Err(execution
            .error_message
            .unwrap_or_else(|| "Command script failed.".to_string()));
    }
    Ok(())
}

#[derive(Debug)]
struct ScriptExecution {
    ok: bool,
    report: Value,
    error_message: Option<String>,
}

fn execute_command_file(
    engine: &mut Engine,
    script: &str,
    inspect_after: Option<&[String]>,
) -> ScriptExecution {
    let commands = match read_command_values(script) {
        Ok(commands) => commands,
        Err(error) => {
            return ScriptExecution {
                ok: false,
                error_message: Some(error.clone()),
                report: json!({
                    "ok": false,
                    "commandCount": 0,
                    "executedCount": 0,
                    "failedIndex": null,
                    "commands": [],
                    "error": {
                        "stage": "read-script",
                        "message": error,
                    },
                }),
            };
        }
    };
    execute_command_values(engine, commands, inspect_after)
}

fn execute_command_values(
    engine: &mut Engine,
    commands: Vec<Value>,
    inspect_after: Option<&[String]>,
) -> ScriptExecution {
    let command_count = commands.len();
    let mut entries = Vec::new();
    let mut executed_count = 0usize;
    for (index, command) in commands.into_iter().enumerate() {
        let command_type = command_type_name(&command);
        match execute_json_command(engine, command.clone()) {
            Ok(engine_result) => {
                executed_count += 1;
                let changed = engine_result
                    .get("changed")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let mut entry = json!({
                    "index": index,
                    "ok": true,
                    "executed": true,
                    "changed": changed,
                    "commandType": command_type,
                    "command": command,
                    "revision": engine_result.get("revision").cloned().unwrap_or(Value::Null),
                    "beforeRevision": engine_result
                        .get("beforeRevision")
                        .cloned()
                        .unwrap_or(Value::Null),
                    "targets": engine_result.get("targets").cloned().unwrap_or_else(|| json!({})),
                    "created": engine_result.get("created").cloned().unwrap_or_else(|| json!({})),
                    "updated": engine_result.get("updated").cloned().unwrap_or_else(|| json!({})),
                    "deleted": engine_result.get("deleted").cloned().unwrap_or_else(|| json!({})),
                    "diagnostics": engine_result
                        .get("diagnostics")
                        .cloned()
                        .unwrap_or_else(|| json!({})),
                    "engineResult": engine_result,
                });
                append_after_snapshot(engine, &mut entry, inspect_after);
                entries.push(entry);
            }
            Err(error) => {
                entries.push(json!({
                    "index": index,
                    "ok": false,
                    "executed": false,
                    "changed": false,
                    "commandType": command_type,
                    "command": command,
                    "error": {
                        "stage": "execute-command",
                        "message": error,
                    },
                }));
                let error_message = entries
                    .last()
                    .and_then(|entry| entry.pointer("/error/message"))
                    .and_then(Value::as_str)
                    .unwrap_or("Command failed.")
                    .to_string();
                let mut report = json!({
                    "ok": false,
                    "commandCount": command_count,
                    "executedCount": executed_count,
                    "failedIndex": index,
                    "commands": entries,
                    "error": {
                        "stage": "execute-command",
                        "message": error_message,
                    },
                });
                append_final_snapshot(engine, &mut report, inspect_after);
                return ScriptExecution {
                    ok: false,
                    report,
                    error_message: Some(error_message),
                };
            }
        }
    }
    let mut report = json!({
        "ok": true,
        "commandCount": command_count,
        "executedCount": executed_count,
        "failedIndex": null,
        "commands": entries,
    });
    append_final_snapshot(engine, &mut report, inspect_after);
    ScriptExecution {
        ok: true,
        report,
        error_message: None,
    }
}

fn empty_script_execution(
    engine: &mut Engine,
    inspect_after: Option<&[String]>,
) -> ScriptExecution {
    let mut report = json!({
        "ok": true,
        "commandCount": 0,
        "executedCount": 0,
        "failedIndex": null,
        "commands": [],
    });
    append_final_snapshot(engine, &mut report, inspect_after);
    ScriptExecution {
        ok: true,
        report,
        error_message: None,
    }
}

fn append_after_snapshot(engine: &mut Engine, entry: &mut Value, inspect_after: Option<&[String]>) {
    let Some(include) = inspect_after else {
        return;
    };
    match inspect_engine(engine, include) {
        Ok(snapshot) => set_report_field(entry, "after", snapshot),
        Err(error) => set_report_field(
            entry,
            "afterError",
            json!({
                "stage": "inspect-after",
                "message": error,
            }),
        ),
    }
}

fn append_final_snapshot(
    engine: &mut Engine,
    report: &mut Value,
    inspect_after: Option<&[String]>,
) {
    let Some(include) = inspect_after else {
        return;
    };
    match inspect_engine(engine, include) {
        Ok(snapshot) => set_report_field(report, "final", snapshot),
        Err(error) => set_report_field(
            report,
            "finalError",
            json!({
                "stage": "inspect-final",
                "message": error,
            }),
        ),
    }
}

fn inspect_engine(engine: &mut Engine, include: &[String]) -> Result<Value, String> {
    let result = execute_json_command(
        engine,
        json!({
            "type": "inspect-document",
            "include": include,
        }),
    )?;
    result
        .get("output")
        .cloned()
        .ok_or_else(|| "inspect command did not return output.".to_string())
}

fn write_optional_document_json(
    execution: &mut ScriptExecution,
    engine: &Engine,
    path: Option<&str>,
    report_key: &str,
) {
    let Some(path) = path else {
        return;
    };
    match document_json(engine).and_then(|text| write_text_output(Some(path), &format!("{text}\n")))
    {
        Ok(()) => set_report_field(
            &mut execution.report,
            report_key,
            json!({
                "ok": true,
                "path": path,
                "format": "json",
            }),
        ),
        Err(error) => {
            execution.ok = false;
            execution.error_message = Some(error.clone());
            set_report_field(
                &mut execution.report,
                report_key,
                json!({
                    "ok": false,
                    "path": path,
                    "error": {
                        "stage": "write-document-json",
                        "message": error,
                    },
                }),
            );
        }
    }
}

fn command_type_name(command: &Value) -> Value {
    command
        .get("type")
        .and_then(Value::as_str)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

fn set_report_field(report: &mut Value, key: &str, value: Value) {
    if !report.is_object() {
        *report = Value::Object(Map::new());
    }
    if let Some(object) = report.as_object_mut() {
        object.insert(key.to_string(), value);
    }
}

fn load_engine_from_file(path: &str) -> Result<Engine, String> {
    let mut service = DesktopDocumentService::default();
    let opened = service.read_document_file(path)?;
    let mut engine = Engine::new();
    match opened.format.as_str() {
        "ccjs" | "ccjz" => engine.load_document_json(&opened.text)?,
        "cdxml" | "cdx" => engine.load_cdxml_document(&opened.text)?,
        "sdf" => engine.load_sdf_document(&opened.text)?,
        "svg" => {
            return Err(
                "SVG is an export format and cannot be opened as an editable document.".to_string(),
            );
        }
        format => return Err(format!("Unsupported input format '{format}'.")),
    }
    Ok(engine)
}

fn write_engine_output(engine: &Engine, path: &str, format: Option<&str>) -> Result<(), String> {
    let format = format
        .map(normalize_format)
        .transpose()?
        .or_else(|| infer_format_from_path(path))
        .ok_or_else(|| "Output format is ambiguous; pass --format.".to_string())?;

    if path == "-" {
        return write_engine_output_to_stdout(engine, &format);
    }

    let mut service = DesktopDocumentService::default();
    match format.as_str() {
        "json" | "ccjs" => service.write_document_file(path, &document_json(engine)?, Some("ccjs")),
        "ccjz" => service.write_document_file(path, &document_json(engine)?, Some("ccjz")),
        "cdxml" => service.write_document_file(path, &engine.document_cdxml(), Some("cdxml")),
        "cdx" => service.write_document_file(path, &engine.document_cdxml(), Some("cdx")),
        "sdf" => service.write_document_file(path, &engine.document_sdf()?, Some("sdf")),
        "svg" => service.write_document_file(path, &engine.document_svg(), Some("svg")),
        _ => Err(format!("Unsupported output format '{format}'.")),
    }?;
    Ok(())
}

fn write_engine_output_to_stdout(engine: &Engine, format: &str) -> Result<(), String> {
    match format {
        "json" | "ccjs" => write_stdout_text(&document_json(engine)?),
        "ccjz" => Err("Writing compressed ccjz to stdout is not supported.".to_string()),
        "cdxml" => write_stdout_text(&engine.document_cdxml()),
        "cdx" => io::stdout()
            .write_all(&engine.document_cdx()?)
            .map_err(|error| error.to_string()),
        "sdf" => write_stdout_text(&engine.document_sdf()?),
        "svg" => write_stdout_text(&engine.document_svg()),
        _ => Err(format!("Unsupported output format '{format}'.")),
    }
}

fn read_command_values(path: &str) -> Result<Vec<Value>, String> {
    let text = if path == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| error.to_string())?;
        input
    } else {
        fs::read_to_string(path).map_err(|error| format!("Failed to read {path}: {error}"))?
    };
    let text = text.trim_start_matches('\u{feff}');
    let value: Value = serde_json::from_str(text)
        .map_err(|error| format!("Invalid command JSON in {path}: {error}"))?;
    match value {
        Value::Array(commands) => Ok(commands),
        Value::Object(_) => Ok(vec![value]),
        _ => Err("Command JSON must be an object or an array of objects.".to_string()),
    }
}

fn execute_json_command(engine: &mut Engine, command: Value) -> Result<Value, String> {
    let result = engine.execute_command_json(&command.to_string())?;
    serde_json::from_str(&result).map_err(|error| error.to_string())
}

fn write_json_value(value: Value, path: Option<&str>, pretty: bool) -> Result<(), String> {
    let text = if pretty {
        serde_json::to_string_pretty(&value)
    } else {
        serde_json::to_string(&value)
    }
    .map_err(|error| error.to_string())?;
    write_text_output(path, &format!("{text}\n"))
}

fn write_text_output(path: Option<&str>, text: &str) -> Result<(), String> {
    match path {
        Some("-") | None => write_stdout_text(text),
        Some(path) => {
            fs::write(path, text).map_err(|error| format!("Failed to write {path}: {error}"))
        }
    }
}

fn write_stdout_text(text: &str) -> Result<(), String> {
    io::stdout()
        .write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

fn document_json(engine: &Engine) -> Result<String, String> {
    engine.document_json().map_err(|error| error.to_string())
}

fn normalize_format(value: &str) -> Result<String, String> {
    let normalized = value.trim().trim_start_matches('.').to_ascii_lowercase();
    let normalized = match normalized.as_str() {
        "json" => "json",
        "ccjs" => "ccjs",
        "ccjz" => "ccjz",
        "cdxml" => "cdxml",
        "cdx" => "cdx",
        "sdf" | "sd" => "sdf",
        "svg" => "svg",
        _ => return Err(format!("Unsupported format '{value}'.")),
    };
    Ok(normalized.to_string())
}

fn infer_format_from_path(path: &str) -> Option<String> {
    if path == "-" {
        return None;
    }
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| normalize_format(extension).ok())
}

fn split_csv(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

fn default_inspect_after() -> Option<Vec<String>> {
    Some(vec![
        "summary".to_string(),
        "objects".to_string(),
        "molecules".to_string(),
    ])
}

fn parse_inspect_after_value(value: &str) -> Option<Vec<String>> {
    let normalized = value.trim().to_ascii_lowercase();
    if matches!(normalized.as_str(), "none" | "off" | "false" | "0") {
        return None;
    }
    let include = split_csv(value);
    if include.is_empty() {
        None
    } else {
        Some(include)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_formats() {
        assert_eq!(normalize_format("json").unwrap(), "json");
        assert_eq!(normalize_format(".cdxml").unwrap(), "cdxml");
        assert_eq!(normalize_format("sd").unwrap(), "sdf");
        assert!(normalize_format("png").is_err());
    }

    #[test]
    fn infers_format_from_output_path() {
        assert_eq!(infer_format_from_path("out.svg").as_deref(), Some("svg"));
        assert_eq!(infer_format_from_path("out.json").as_deref(), Some("json"));
        assert_eq!(infer_format_from_path("-"), None);
    }

    #[test]
    fn command_json_accepts_single_object_or_array() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-single.json",
            std::process::id()
        ));
        fs::write(&path, r#"{ "type": "inspect-document" }"#).unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            1
        );
        let _ = fs::remove_file(path);

        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-array.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"[{ "type": "inspect-document" }, { "type": "export-document", "format": "svg" }]"#,
        )
        .unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            2
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn command_json_accepts_utf8_bom() {
        let path = std::env::temp_dir().join(format!(
            "chemcore-cli-command-test-{}-bom.json",
            std::process::id()
        ));
        fs::write(&path, "\u{feff}{ \"type\": \"inspect-document\" }").unwrap();
        assert_eq!(
            read_command_values(path.to_str().unwrap()).unwrap().len(),
            1
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn execution_report_includes_after_snapshot_for_success() {
        let mut engine = Engine::new();
        let include = default_inspect_after().unwrap();
        let execution = execute_command_values(
            &mut engine,
            vec![json!({
                "type": "add-bond",
                "begin": { "x": 100.0, "y": 120.0 },
                "end": { "x": 140.0, "y": 120.0 },
                "order": 1,
                "variant": "single",
            })],
            Some(&include),
        );

        assert!(execution.ok);
        assert_eq!(execution.report["ok"], true);
        assert_eq!(execution.report["commandCount"], 1);
        assert_eq!(execution.report["executedCount"], 1);
        assert_eq!(execution.report["commands"][0]["ok"], true);
        assert_eq!(execution.report["commands"][0]["executed"], true);
        assert_eq!(execution.report["commands"][0]["changed"], true);
        assert!(execution.report["commands"][0]["after"]["molecules"].is_array());
        assert!(execution.report["final"]["molecules"].is_array());
    }

    #[test]
    fn execution_report_records_failed_command_without_saving() {
        let mut engine = Engine::new();
        let include = default_inspect_after().unwrap();
        let execution = execute_command_values(
            &mut engine,
            vec![
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 140.0, "y": 120.0 },
                    "order": 1,
                    "variant": "single",
                }),
                json!({
                    "type": "add-bond",
                    "begin": { "x": 100.0, "y": 120.0 },
                    "end": { "x": 180.0, "y": 120.0 },
                    "order": 1,
                    "variant": "not-a-bond-style",
                }),
            ],
            Some(&include),
        );

        assert!(!execution.ok);
        assert_eq!(execution.report["ok"], false);
        assert_eq!(execution.report["commandCount"], 2);
        assert_eq!(execution.report["executedCount"], 1);
        assert_eq!(execution.report["failedIndex"], 1);
        assert_eq!(execution.report["commands"][0]["executed"], true);
        assert_eq!(execution.report["commands"][1]["executed"], false);
        assert_eq!(
            execution.report["commands"][1]["error"]["stage"],
            "execute-command"
        );
        assert!(execution.report["final"]["molecules"].is_array());
    }
}
