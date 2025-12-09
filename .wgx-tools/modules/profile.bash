#!/usr/bin/env bash

# shellcheck shell=bash

PROFILE_FILE=""
PROFILE_VERSION=""
WGX_REQUIRED_RANGE=""
WGX_REQUIRED_MIN=""
WGX_PROFILE_LOADED=""

export WGX_REPO_KIND=""
export WGX_DIR_WEB=""
export WGX_DIR_API=""
export WGX_DIR_DATA=""

# shellcheck disable=SC2034
WGX_AVAILABLE_CAPS=(task-array status-dirs tasks-json validate env-defaults env-overrides workflows)

declare -ga WGX_REQUIRED_CAPS=()
declare -ga WGX_ENV_KEYS=()

declare -gA WGX_ENV_BASE_MAP=()
declare -gA WGX_ENV_DEFAULT_MAP=()
declare -gA WGX_ENV_OVERRIDE_MAP=()

# shellcheck disable=SC2034
declare -ga WGX_TASK_ORDER=()
declare -gA WGX_TASK_CMDS=()
declare -gA WGX_TASK_DESC=()
declare -gA WGX_TASK_GROUP=()
declare -gA WGX_TASK_SAFE=()

declare -gA WGX_WORKFLOW_TASKS=()

profile::_reset() {
  PROFILE_VERSION=""
  WGX_REQUIRED_RANGE=""
  WGX_REQUIRED_MIN=""
  WGX_REPO_KIND=""
  WGX_DIR_WEB=""
  WGX_DIR_API=""
  WGX_DIR_DATA=""
  WGX_REQUIRED_CAPS=()
  WGX_ENV_KEYS=()
  # shellcheck disable=SC2034
  WGX_TASK_ORDER=()
  WGX_ENV_BASE_MAP=()
  WGX_ENV_DEFAULT_MAP=()
  WGX_ENV_OVERRIDE_MAP=()
  WGX_TASK_CMDS=()
  WGX_TASK_DESC=()
  WGX_TASK_GROUP=()
  WGX_TASK_SAFE=()
  WGX_WORKFLOW_TASKS=()
  WGX_PROFILE_LOADED=""
}

profile::_detect_file() {
  PROFILE_FILE=""
  local root="${WGX_TARGET_ROOT:-.}"
  local base
  for base in ".wgx/profile.yml" ".wgx/profile.yaml" ".wgx/profile.json" ".wgx/profile.example.yml"; do
    local candidate="${root%/}/$base"
    if [[ -f "$candidate" ]]; then
      PROFILE_FILE="$candidate"
      return 0
    fi
  done
  return 1
}

profile::_have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

profile::_module_dir() {
  cd "$(dirname "${BASH_SOURCE[0]}")" && pwd
}

profile::_abspath() {
  local p="$1" resolved=""
  local module_dir
  module_dir="$(profile::_module_dir)"
  if profile::_have_cmd python3; then
    if resolved="$(python3 "${module_dir}/abspath.py" "$p" 2>/dev/null)"; then
      if [[ -n $resolved ]]; then
        printf '%s\n' "$resolved"
        return 0
      fi
    fi
  fi
  if command -v readlink >/dev/null 2>&1; then
    resolved="$(readlink -f -- "$p" 2>/dev/null || true)"
    if [[ -n $resolved ]]; then
      printf '%s\n' "$resolved"
    else
      printf '%s\n' "$p"
    fi
    return 0
  fi
  printf '%s\n' "$p"
}

profile::_normalize_task_name() {
  local name="$1"
  name="${name// /-}"
  while [[ $name == *--* ]]; do
    name="${name//--/-}"
  done
  printf '%s' "${name,,}"
}

profile::_python_parse() {
  local file="$1" output
  local module_dir
  module_dir="$(profile::_module_dir)"
  profile::_have_cmd python3 || return 1
  output="$(python3 "${module_dir}/profile_parser.py" "$file")"
  local status=$?
  if ((status != 0)); then
    return $status
  fi

  if [[ -z $output ]]; then
    return 0
  fi

  local line
  while IFS= read -r line; do
    [[ -n $line ]] || continue
    eval "$line"
  done <<<"$output"

  return 0
}

profile::_decode_json_array() {
  local json_payload="$1"
  local module_dir
  module_dir="$(profile::_module_dir)"
  profile::_have_cmd python3 || return 1
  python3 "${module_dir}/json_decode.py" "$json_payload"
}

profile::_collect_env_keys() {
  local -A seen=()
  WGX_ENV_KEYS=()
  local key
  for key in "${!WGX_ENV_DEFAULT_MAP[@]}"; do
    if [[ -z ${seen[$key]:-} ]]; then
      seen[$key]=1
      WGX_ENV_KEYS+=("$key")
    fi
  done
  for key in "${!WGX_ENV_BASE_MAP[@]}"; do
    if [[ -z ${seen[$key]:-} ]]; then
      seen[$key]=1
      WGX_ENV_KEYS+=("$key")
    fi
  done
  for key in "${!WGX_ENV_OVERRIDE_MAP[@]}"; do
    if [[ -z ${seen[$key]:-} ]]; then
      seen[$key]=1
      WGX_ENV_KEYS+=("$key")
    fi
  done
}

profile::has_manifest() {
  profile::_detect_file
}

profile::load() {
  local file="${1:-}"
  if [[ -n $file ]]; then
    [[ -f $file ]] || return 1
    PROFILE_FILE="$file"
  else
    profile::_detect_file || return 1
    file="$PROFILE_FILE"
  fi
  local _norm_file
  _norm_file="$(profile::_abspath "$file")"
  if [[ ${WGX_PROFILE_LOADED:-} == "$_norm_file" ]]; then
    return 0
  fi
  profile::_reset
  local status=1
  if [[ $file == *.yml || $file == *.yaml || $file == *.json ]]; then
    if profile::_python_parse "$file"; then
      status=0
    else
      local rc=$?
      if ((rc == 3)); then
        status=3
      fi
    fi
  fi
  if ((status != 0)); then
    # The Python parser failed, so we can't continue.
    # The flat yaml parser has been removed.
    return 1
  fi
  profile::_collect_env_keys
  WGX_PROFILE_LOADED="$_norm_file"
  return 0
}

profile::ensure_loaded() {
  if ! profile::has_manifest; then
    profile::_reset
    PROFILE_FILE=""
    return 1
  fi
  profile::load "$PROFILE_FILE"
}

profile::available_caps() {
  printf '%s\n' "${WGX_AVAILABLE_CAPS[@]}"
}

profile::ensure_version() {
  [[ -z ${WGX_VERSION:-} ]] && return 0
  if [[ -z $WGX_REQUIRED_RANGE && -z $WGX_REQUIRED_MIN ]]; then
    return 0
  fi
  if ! declare -F semver_norm >/dev/null 2>&1; then
    source "${WGX_DIR:-.}/modules/semver.bash"
  fi
  local have="${WGX_VERSION:-0.0.0}"
  if [[ -n $WGX_REQUIRED_RANGE ]]; then
    if [[ $WGX_REQUIRED_RANGE == ^* ]]; then
      if ! semver_in_caret_range "$have" "$WGX_REQUIRED_RANGE"; then
        warn "wgx version ${have} outside required range ${WGX_REQUIRED_RANGE}"
        return 1
      fi
    else
      if ! semver_ge "$have" "$WGX_REQUIRED_RANGE"; then
        warn "wgx version ${have} < required ${WGX_REQUIRED_RANGE}"
        return 1
      fi
    fi
  fi
  if [[ -n $WGX_REQUIRED_MIN ]]; then
    if ! semver_ge "$have" "$WGX_REQUIRED_MIN"; then
      warn "wgx version ${have} < required minimum ${WGX_REQUIRED_MIN}"
      return 1
    fi
  fi
  return 0
}

profile::_task_keys() {
  if ((${#WGX_TASK_CMDS[@]} == 0)); then
    return 0
  fi
  local key
  for key in "${!WGX_TASK_CMDS[@]}"; do
    printf '%s\n' "$(profile::_normalize_task_name "$key")"
  done | sort -u
}

profile::tasks() {
  profile::ensure_loaded || return 1
  profile::_task_keys
}

profile::_task_safe() {
  local key="$1"
  printf '%s' "${WGX_TASK_SAFE[$key]:-0}"
}

profile::_task_desc() {
  local key="$1"
  printf '%s' "${WGX_TASK_DESC[$key]:-}"
}

profile::_task_group() {
  local key="$1"
  printf '%s' "${WGX_TASK_GROUP[$key]:-}"
}

profile::_task_spec() {
  local key="$1"
  printf '%s' "${WGX_TASK_CMDS[$key]:-}"
}

# Hilfsfunktion: Shell-ähnliche Einfach-Quotes für Ausgabe
profile::_quote_arg_for_display() {
  local s=$1
  # Einfaches ' wird in der klassischen Shell-Notierung als '\'' dargestellt
  s=${s//\'/\'\\\'\'}
  printf "'%s'" "$s"
}

profile::_shell_quote() {
  local value="$1"
  if [[ -z $value ]]; then
    printf "''"
    return
  fi
  if [[ $value =~ ^[A-Za-z0-9_@%+=:,./-]+$ ]]; then
    printf '%s' "$value"
    return
  fi
  printf "'%s'" "${value//\'/\'\\\'\'}"
}

profile::_dry_run_quote() {
  profile::_shell_quote "$1"
}

profile::tasks_json() {
  profile::ensure_loaded || return 1
  local safe_only="${1:-0}" include_groups="${2:-0}"
  if ! declare -F json_escape >/dev/null 2>&1; then
    source "${WGX_DIR:-.}/modules/json.bash"
  fi
  local sep=""
  printf '{"tasks":['
  local key name safe desc group
  local -A groups=()
  for name in $(profile::_task_keys); do
    key="$name"
    safe="$(profile::_task_safe "$key")"
    if ((safe_only)) && [[ "$safe" != "1" ]]; then
      continue
    fi
    desc="$(profile::_task_desc "$key")"
    group="$(profile::_task_group "$key")"
    printf '%s{"name":"%s","desc":"%s","group":"%s","safe":%s}' \
      "$sep" "$(json_escape "$name")" "$(json_escape "$desc")" "$(json_escape "$group")" \
      "$([[ $safe == 1 ]] && echo true || echo false)"
    sep=','
    if ((include_groups)) && [[ -n $group ]]; then
      groups["$group"]=1
    fi
  done
  printf ']'
  if ((include_groups)); then
    printf ',"groups":['
    sep=""
    local g
    for g in "${!groups[@]}"; do
      printf '%s"%s"' "$sep" "$(json_escape "$g")"
      sep=','
    done
    printf ']'
  fi
  printf '}'
  printf '\n'
}

profile::env_apply() {
  profile::ensure_loaded || return 1
  local envs=()
  local key
  for key in "${!WGX_ENV_DEFAULT_MAP[@]}"; do
    envs+=("${key}=${WGX_ENV_DEFAULT_MAP[$key]}")
  done
  for key in "${!WGX_ENV_BASE_MAP[@]}"; do
    envs+=("${key}=${WGX_ENV_BASE_MAP[$key]}")
  done
  for key in "${!WGX_ENV_OVERRIDE_MAP[@]}"; do
    envs+=("${key}=${WGX_ENV_OVERRIDE_MAP[$key]}")
  done
  if ((${#envs[@]})); then
    printf '%s\n' "${envs[@]}"
  fi
}

profile::run_task() {
  local name="${1-}"
  if [[ -z $name ]]; then
    printf 'Usage: wgx run <task>\n\nAvailable tasks:\n' >&2
    profile::tasks | sed 's/^/  /' >&2
    return 1
  fi
  shift || true

  local key
  key="$(profile::_normalize_task_name "$name")"
  local spec
  spec="$(profile::_task_spec "$key")"
  if [[ -z $spec ]]; then
    printf 'Task not defined: %s\n' "$key" >&2
    return 1
  fi
  local -a envs=()
  mapfile -t envs < <(profile::env_apply)
  local dryrun="${DRYRUN:-0}"
  local args=()
  local passthrough=0
  while (($#)); do
    if ((passthrough)); then
      args+=("$1")
    else
      if [[ $1 == -- ]]; then
        passthrough=1
      else
        args+=("$1")
      fi
    fi
    shift || true
  done

  # Menschlich lesbare Repräsentation für Tests/Debug:
  local raw_cmd base_cmd
  case "$spec" in
  STR:*)
    base_cmd="${spec#STR:}" # z.B. "echo 'a # b'"
    raw_cmd="STR:${base_cmd}"
    ;;
  ARR:*)
    # Für ARR ist die Erwartung in den Tests meist weniger hart; falls nötig:
    # base_cmd wird hier notfalls aus der internen Darstellung rekonstruiert.
    ;;
  esac

  if ((${#args[@]} > 0)); then
    for _arg in "${args[@]}"; do
      raw_cmd+=" $(profile::_quote_arg_for_display "$_arg")"
    done
  fi

  if ((dryrun)); then
    echo "raw_cmd=${raw_cmd}"
  fi

  # Respect WGX_TARGET_ROOT as the working directory for the task execution
  local workdir="${WGX_TARGET_ROOT:-.}"

  case "$spec" in
  ARRJSON:*)
    local payload_json="${spec#ARRJSON:}"
    local -a cmd=()
    if [[ -n $payload_json ]]; then
      if ! mapfile -t cmd < <(profile::_decode_json_array "$payload_json"); then
        return 1
      fi
    fi
    if ((${#cmd[@]} == 0)); then
      printf 'wgx: empty command for task %q\n' "$key" >&2
      return 2
    fi
    if ((dryrun)); then
      local out='[DRY-RUN]'
      local item
      for item in "${envs[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      for item in "${cmd[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      for item in "${args[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      printf '%s\n' "$out"
      return 0
    fi
    (
      cd "$workdir" || return 1
      ((${#envs[@]})) && export "${envs[@]}"
      exec "${cmd[@]}" "${args[@]}"
    )
    ;;
  ARR:*)
    local payload="${spec#ARR:}"
    local -a cmd=()
    if [[ -n $payload ]]; then
      read -r -a cmd <<<"$payload"
    fi
    if ((${#cmd[@]} == 0)); then
      printf 'wgx: empty command for task %q\n' "$key" >&2
      return 2
    fi
    if ((dryrun)); then
      local out='[DRY-RUN]'
      local item
      for item in "${envs[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      for item in "${cmd[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      for item in "${args[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      printf '%s\n' "$out"
      return 0
    fi
    (
      cd "$workdir" || return 1
      ((${#envs[@]})) && export "${envs[@]}"
      exec "${cmd[@]}" "${args[@]}"
    )
    ;;
  STR:*)
    local command="${spec#STR:}"
    if ((dryrun)); then
      local out='[DRY-RUN]'
      local item
      for item in "${envs[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      if [[ -n $command ]]; then
        out+=" $command"
      fi
      for item in "${args[@]}"; do
        out+=" $(profile::_dry_run_quote "$item")"
      done
      printf '%s\n' "$out"
      return 0
    fi
    (
      cd "$workdir" || return 1
      ((${#envs[@]})) && export "${envs[@]}"
      if ((${#args[@]})); then
        local extra=""
        local arg
        for arg in "${args[@]}"; do
          extra+=" "
          extra+="$(printf '%q' "$arg")"
        done
        exec bash -lc "$command$extra"
      else
        exec bash -lc "$command"
      fi
    )
    ;;
  *)
    return 1
    ;;
  esac
}

profile::validate_manifest() {
  profile::ensure_loaded || return 1
  local -n _errors_ref=$1
  local -n _missing_caps_ref=$2
  _errors_ref=()
  _missing_caps_ref=()
  if [[ -z $PROFILE_VERSION ]]; then
    _errors_ref+=("missing_apiVersion")
  else
    case "$PROFILE_VERSION" in
    v1 | 1 | 1.0 | v1.0) ;;
    v1.1 | 1.1) ;;
    *)
      _errors_ref+=("unsupported_apiVersion")
      ;;
    esac
  fi
  if ((${#WGX_TASK_CMDS[@]} == 0)); then
    _errors_ref+=("no_tasks")
  fi
  local key spec
  for key in "${!WGX_TASK_CMDS[@]}"; do
    spec="${WGX_TASK_CMDS[$key]}"
    [[ -n $spec ]] || _errors_ref+=("task_missing_command:${key}")
  done
  if ! profile::ensure_version; then
    _errors_ref+=("version_mismatch")
  fi
  local -a missing=()
  local cap
  local -A available_map=()
  for cap in "${WGX_AVAILABLE_CAPS[@]}"; do
    available_map[$cap]=1
  done
  for cap in "${WGX_REQUIRED_CAPS[@]}"; do
    if [[ -z ${available_map[$cap]:-} ]]; then
      missing+=("$cap")
    fi
  done
  if ((${#missing[@]})); then
    _missing_caps_ref=("${missing[@]}")
    _errors_ref+=("missing_capabilities")
  fi
  local wf tasks wf_task
  for wf in "${!WGX_WORKFLOW_TASKS[@]}"; do
    tasks="${WGX_WORKFLOW_TASKS[$wf]}"
    [[ -z $tasks ]] && continue
    for wf_task in $tasks; do
      local normalised
      normalised="$(profile::_normalize_task_name "$wf_task")"
      if [[ -z ${WGX_TASK_CMDS[$normalised]:-} ]]; then
        _errors_ref+=("workflow_missing_task:${wf}:${wf_task}")
      fi
    done
  done
}

profile::check_workflows() {
  local module_dir
  module_dir="$(profile::_module_dir)"
  local script_path="${module_dir}/../scripts/check_workflows.py"
  if [[ -f "$script_path" ]]; then
    profile::_have_cmd python3 || return 1
    python3 "$script_path"
  else
    warn "check_workflows.py not found at $script_path"
    return 1
  fi
}

profile::_auto_init() {
  if profile::has_manifest; then
    if profile::load "$PROFILE_FILE"; then
      profile::ensure_version || true
    else
      warn "profile manifest could not be loaded"
    fi
  fi
}
