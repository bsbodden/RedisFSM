#[macro_use] extern crate redis_module;

use redis_module::{Context, RedisError, RedisResult, RedisString, NextArg};

fn fsm_create(_: &Context, args: Vec<RedisString>) -> RedisResult {
  let args = args.into_iter().skip(1);
  let args_card = args.len();

}
