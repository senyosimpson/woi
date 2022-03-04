@default:
  just --list

# Runs cargo clippy
check:
  cargo clippy --all-targets -- -A clippy::module_inception -A clippy::new_ret_no_self -A clippy::zero_ptr