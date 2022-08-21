#[macro_use] extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString, NextArg};
use serde::{Deserialize, Serialize};
use redis_module::native_types::RedisType;
use redis_module::raw::RedisModuleTypeMethods;
use std::os::raw::c_void;
use serde_json::json;

//////////////////////////////////////////////////////

#[derive(Serialize, Deserialize, Debug)]
struct Event {
  name: String,
  from: Vec<String>,
  to: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct StateMachine {
  name: String,
  prefix: String,
  field: String,
  states: Vec<String>,
  events: Vec<Event>,
}

unsafe extern "C" fn free(value: *mut c_void) {
  Box::from_raw(value.cast::<StateMachine>());
}

//////////////////////////////////////////////////////

pub const REDIS_FSM_TYPE_NAME: &str = "Redis-FSM";
pub const REDIS_FSM_TYPE_VERSION: i32 = 1;

pub static REDIS_FSM_TYPE: RedisType = RedisType::new(
  REDIS_FSM_TYPE_NAME,
  REDIS_FSM_TYPE_VERSION,
  RedisModuleTypeMethods {
    version: redis_module::TYPE_METHOD_VERSION,
    rdb_load: None,
    rdb_save: None,
    aof_rewrite: None,
    free: Some(free),
    mem_usage: None,
    digest: None,
    aux_load: None,
    aux_save: None,
    aux_save_triggers: 0,
    free_effort: None,
    unlink: None,
    copy: None,
    defrag: None,
  },
);

//////////////////////////////////////////////////////

fn fsm_create(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
  let mut args = args.into_iter().skip(1);
  let key = args.next_arg()?;
  let redis_key = ctx.open_key_writable(&key);

  let fsm_json = json!({
    "name": "JobFSM",
    "prefix": "job:",
    "field": "state",
    "states": [
      "sleeping",
      "running",
      "cleaning"
    ],
    "events": [
      {
        "name": "run",
        "from": [
          "sleeping"
        ],
        "to": "running"
      },
      {
        "name": "clean",
        "from": [
          "running"
        ],
        "to": "cleaning"
      },
      {
        "name": "sleep",
        "from": [
          "running",
          "cleaning"
        ],
        "to": "sleeping"
      }
    ]
  });

  let fsm: StateMachine = serde_json::from_str(&fsm_json.to_string())?;
  redis_key.set_value(&REDIS_FSM_TYPE, fsm)?;
  let src = args.into_iter().next_string()?;
  let greet = format!("👋 Hello {}", src);
  let response = Vec::from(greet);

  return Ok(response.into());
}

//////////////////////////////////////////////////////

redis_module! {
  name: "fsm",
  version: 1,
  data_types: [],
  commands: [
    ["fsm.create", fsm_create, "", 0, 0, 0],
  ],
}
