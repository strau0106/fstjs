use std::collections::HashMap;
use fstapi::{file_type, var_type, Attr, Hier, Reader};
use napi::{Env, JsNumber, JsObject, JsString, Result};
use napi_derive::napi;
use std::path::Path;

trait ToNapiError {
  fn to_napi_error(self) -> napi::Error;
}

impl ToNapiError for fstapi::Error {
  fn to_napi_error(self) -> napi::Error {
    napi::Error::new(napi::Status::GenericFailure, format!("FST error: {}", self))
  }
}

#[napi]
pub struct FstEnum {
  name: String,
  length: usize,
  map: HashMap<u8, String>,
}

pub struct FstJsReaderContent {
  reader: Reader,
  enums: HashMap<String, FstEnum>,
}

#[napi] 
pub struct FstJsReader {
  content: FstJsReaderContent,
}

#[napi]
impl FstJsReader {
  /// Constructor: Create a new FST reader
  #[napi(constructor)]
  pub fn new(file: String) -> Result<Self> {
    let path = Path::new(&file);
    let mut reader = Reader::open(path).map_err(|e| e.to_napi_error())?;      
    let mut enums = HashMap::<String, FstEnum>::new();

    for hier in reader.hiers() {
      if let Hier::AttrBegin(attr) = hier {
      if let Some(enum_val) = FstJsReader::hier_to_enum(attr) {
        enums.insert(enum_val.name.clone(), enum_val);
      }
      }
    }

    Ok(Self { content: FstJsReaderContent {reader, enums }})
  }

  fn hier_to_enum(attr: Attr) -> Option<FstEnum> {
    if let Ok(attr_string) = attr.name() {
      let attr_vec: Vec<&str> = attr_string.split_whitespace().collect();
      if attr_vec.len() >= 4 {
        let length = attr_vec[1].parse().unwrap_or(0);
        let mut map = HashMap::new();
        let names = &attr_vec[2..2 + length];
        let values = &attr_vec[2 + length..];

        for (name, val) in names.iter().zip(values.iter()) {
          if let Ok(val) = isize::from_str_radix(val, 2) {
            map.insert(val as u8, name.to_string());
          }
        }
        return Some(FstEnum {
          name: attr_vec[0].to_string(),
          length,
          map,
        
        });
      }
    }
    None
  }
  /// Read all variable names from the FST file
  #[napi]
  pub fn read(&mut self, env: Env) -> Result<JsString> {
    let mut output = String::new();
    for var in self.content.reader.vars() {
      let (name, _) = var.map_err(|e| e.to_napi_error())?;
      output.push_str(&format!("{name}\n"));
    }
    env.create_string(&output)
  }

  /// Get the value of a variable at a specific time
  #[napi]
  pub fn get_var_value_at_time(
    &mut self,
    env: Env,
    var_name: String,
    time: i64,

  ) -> Result<JsNumber> {
    let mut var_handle = None;

    for var in self.content.reader.vars() {
      let (name, var) = var.map_err(|e| e.to_napi_error())?;
      if name.eq(&var_name) {
        var_handle = Some(var.handle());
        break;
      }
    }

    if let Some(handle) = var_handle {
      if let Some(val) = self.content.reader.get_value_from_handle_at_time(time as u64, handle) {
        return env.create_int32(isize::from_str_radix(&val, 2).unwrap() as i32);
      }
    }
    Err(napi::Error::new(napi::Status::InvalidArg, "Not Found"))
  }
  #[napi]
  pub fn get_var_enum_value_at_time(
    &mut self,
    env: Env,
    var_name: String,
    enum_name: String,
    time: i64,

  ) -> Result<JsString> {
    let mut var_handle = None;

    for var in self.content.reader.vars() {
      let (name, var) = var.map_err(|e| e.to_napi_error())?;
      if name.eq(&var_name) {
        var_handle = Some(var.handle());
        break;
      }
    }

    if let Some(handle) = var_handle {
      if let Some(val) = self.content.reader.get_value_from_handle_at_time(time as u64, handle) {
        let enum_map = self.content.enums.get(&enum_name).ok_or_else(|| napi::Error::new(napi::Status::InvalidArg, "Enum not found"))?;
        let key = isize::from_str_radix(&val, 2).map_err(|_| napi::Error::new(napi::Status::InvalidArg, "Invalid value"))? as u8;
        let enum_value = enum_map.map.get(&key).ok_or_else(|| napi::Error::new(napi::Status::InvalidArg, "Value not found in enum"))?;
        return env.create_string(enum_value);
      }
    }
    Err(napi::Error::new(napi::Status::InvalidArg, "Not Found"))
  }
  /// Get the next time change for a variable
  #[napi]
  pub fn get_next_time_change(
    &mut self,
    env: Env,
    var_name: String,
    start_time: i32,
  ) -> Result<JsNumber> {
    let mut var_handle = None;

    for var in self.content.reader.vars() {
      let (name, var) = var.map_err(|e| e.to_napi_error())?;
      if name == var_name {
        var_handle = Some(var.handle());
        break;
      }
    }

    if let Some(handle) = var_handle {
      let time = self
          .content
          .reader
          .get_next_time_change(start_time as u64, handle)
          .map_err(|e| e.to_napi_error())?;
      return env.create_uint32(time as u32);
    }
    Err(napi::Error::new(napi::Status::InvalidArg, "Not Found"))
  }

  /// Get the timescale of the FST file
  #[napi]
  pub fn get_timescale(&self, env: Env) -> Result<JsString> {
    let timescale = self.content.reader.timescale_str();
    if let Some(ts) = timescale {
      env.create_string(ts)
    } else {
      env.create_string("Unknown")
    }
  }

  /// Get the timezero of the FST file
  #[napi]
  pub fn get_timezero(&self) -> Result<i64> {
    Ok(self.content.reader.timezero())
  }

  /// Get metadata about the FST file
  #[napi]
  pub fn get_metadata(&mut self, env: Env) -> Result<JsObject> {
    let mut obj = env.create_object()?;

    let date = self.content.reader.date().map_err(|e| e.to_napi_error())?;
    obj.set_named_property("date", env.create_string(date)?)?;

    let version = self.content.reader.version().map_err(|e| e.to_napi_error())?;
    obj.set_named_property("version", env.create_string(version)?)?;

    obj.set_named_property("start_time", self.content.reader.start_time() as i64)?;
    obj.set_named_property("end_time", self.content.reader.end_time() as i64)?;

    let file_type = match self.content.reader.file_type() {
      file_type::VERILOG => "Verilog",
      file_type::VHDL => "VHDL",
      file_type::VERILOG_VHDL => "Verilog/VHDL",
      _ => "Unknown",
    };
    obj.set_named_property("file_type", env.create_string(file_type)?)?;

    let timescale = self.content.reader.timescale_str().unwrap_or("Unknown");
    obj.set_named_property("timescale", env.create_string(timescale)?)?;

    obj.set_named_property("timezero", self.content.reader.timezero())?;
    obj.set_named_property("scope_count", self.content.reader.scope_count() as i64)?;
    obj.set_named_property("var_count", self.content.reader.var_count() as i64)?;
    obj.set_named_property("alias_count", self.content.reader.alias_count() as i64)?;

    Ok(obj)
  }

  /// Get variable information
  #[napi]
  pub fn get_variable_info(&mut self, env: Env, var_name: String) -> Result<JsObject> {
    let mut var_type = None;

    for var in self.content.reader.vars() {
      let (name, var) = var.map_err(|e| e.to_napi_error())?;
      if name == var_name {
        var_type = Some(var.ty());
        break;
      }
    }

    let mut obj = env.create_object()?;

    if let Some(ty) = var_type {
      obj.set_named_property(
        "type",
        env.create_string(match ty {
          var_type::VCD_EVENT => "VcdEvent",
          var_type::VCD_INTEGER => "VcdInteger",
          var_type::VCD_PARAMETER => "VcdParameter",
          var_type::VCD_REAL => "VcdReal",
          var_type::VCD_REG => "VcdReg",
          _ => "Unknown",
        })?,
      )?;
    }

    Ok(obj)
  }
}
