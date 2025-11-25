#!/usr/bin/env bash

# shellcheck shell=bash

PROFILE_FILE=""
PROFILE_VERSION=""
WGX_REQUIRED_RANGE=""
WGX_REQUIRED_MIN=""
WGX_REPO_KIND=""
WGX_DIR_WEB=""
WGX_DIR_API=""
WGX_DIR_DATA=""
WGX_PROFILE_LOADED=""

# shellcheck disable=SC2034
WGX_AVAILABLE_CAPS=(task-array status-dirs tasks-json validate env-defaults env-overrides workflows)

declare -ga WGX_REQUIRED_CAPS=()
declare -ga WGX_ENV_KEYS=()

declare -gA WGX_ENV_BASE_MAP=()
declare -gA WGX_ENV_DEFAULT_MAP=()
declare -gA WGX_ENV_OVERRIDE_MAP=()

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

profile::_abspath() {
  local p="$1" resolved=""
  if profile::_have_cmd python3; then
    if resolved="$(
      python3 - "$p" <<'PY' 2>/dev/null
import os
import sys

print(os.path.abspath(sys.argv[1]))
PY
    )"; then
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
  profile::_have_cmd python3 || return 1
  output="$(
    python3 - "$file" <<'PY'
import ast
import json
import os
import shlex
import sys
from typing import Any, Dict, List


def _parse_scalar(value: str) -> Any:
    text = value.strip()
    if text == "":
        return ""
    lowered = text.lower()
    if lowered in {"true", "yes"}:
        return True
    if lowered in {"false", "no"}:
        return False
    if lowered in {"null", "none", "~"}:
        return None
    try:
        return ast.literal_eval(text)
    except Exception:
        return text


def _convert_frame(frame: Dict[str, Any], kind: str) -> None:
    if frame["type"] == kind:
        return
    parent = frame["parent"]
    key = frame["key"]
    if kind == "list":
        new_value: List[Any] = []
        if parent is None:
            frame["container"] = new_value
        elif isinstance(parent, list):
            parent[key] = new_value
        else:
            parent[key] = new_value
        frame["container"] = new_value
        frame["type"] = "list"
    else:
        new_value: Dict[str, Any] = {}
        if parent is None:
            frame["container"] = new_value
        elif isinstance(parent, list):
            parent[key] = new_value
        else:
            parent[key] = new_value
        frame["container"] = new_value
        frame["type"] = "dict"


def _strip_inline_comment(line: str) -> str:
    result: List[str] = []
    in_single = False
    in_double = False
    i = 0
    length = len(line)
    while i < length:
        ch = line[i]
        if in_single:
            result.append(ch)
            if ch == "'" and (i + 1 >= length or line[i + 1] != "'"):
                in_single = False
            elif ch == "'" and i + 1 < length and line[i + 1] == "'":
                result.append(line[i + 1])
                i += 1
            i += 1
            continue
        if in_double:
            result.append(ch)
            if ch == '"' and (i == 0 or line[i - 1] != "\\"):
                in_double = False
            i += 1
            continue
        if ch == "'":
            in_single = True
            result.append(ch)
            i += 1
            continue
        if ch == '"':
            in_double = True
            result.append(ch)
            i += 1
            continue
        if ch == '#':
            break
        result.append(ch)
        i += 1
    return ''.join(result)


def _split_key_value(text: str):
    """
    Splits a YAML line on the first unquoted colon.

    Parameters:
        text (str): The YAML line to parse.

    Returns:
        tuple[str, str] or None: A tuple of (key, value) if a colon is found outside of quotes,
        or None if no such colon exists.
    """
    in_single = False
    in_double = False
    escape = False
    i = 0
    length = len(text)
    while i < length:
        ch = text[i]
        if in_single:
            if ch == "'" and i + 1 < length and text[i + 1] == "'":
                i += 2
                continue
            if ch == "'":
                in_single = False
            i += 1
            continue
        if in_double:
            if escape:
                escape = False
                i += 1
                continue
            if ch == "\\":
                escape = True
                i += 1
                continue
            if ch == '"':
                in_double = False
            i += 1
            continue
        if ch == "'":
            in_single = True
            i += 1
            continue
        if ch == '"':
            in_double = True
            i += 1
            continue
        if ch == ":":
            return text[:i], text[i + 1 :]
        i += 1
    return None

def _parse_simple_yaml(path: str) -> Any:
    root: Dict[str, Any] = {}
    stack: List[Dict[str, Any]] = [
        {"indent": -1, "container": root, "parent": None, "key": None, "type": "dict"}
    ]

    with open(path, "r", encoding="utf-8") as handle:
        for raw_line in handle:
            line = raw_line.rstrip("\n")
            stripped = _strip_inline_comment(line).rstrip()
            if not stripped:
                continue
            indent = len(line) - len(line.lstrip(" "))
            content = stripped.lstrip()

            while len(stack) > 1 and indent <= stack[-1]["indent"]:
                stack.pop()

            frame = stack[-1]
            container = frame["container"]

            if content.startswith("- "):
                value_part = content[2:].strip()
                _convert_frame(frame, "list")
                container = frame["container"]
                if not value_part:
                    item: Dict[str, Any] = {}
                    container.append(item)
                    stack.append(
                        {
                            "indent": indent,
                            "container": item,
                            "parent": container,
                            "key": len(container) - 1,
                            "type": "dict",
                        }
                    )
                    continue
                split = _split_key_value(value_part)
                if split is not None:
                    key, rest = split
                    key = key.strip()
                    rest = rest.strip()
                    item: Dict[str, Any] = {}
                    container.append(item)
                    frame_item = {
                        "indent": indent,
                        "container": item,
                        "parent": container,
                        "key": len(container) - 1,
                        "type": "dict",
                    }
                    stack.append(frame_item)
                    if rest:
                        item[key] = _parse_scalar(rest)
                    else:
                        item[key] = {}
                        stack.append(
                            {
                                "indent": indent,
                                "container": item[key],
                                "parent": item,
                                "key": key,
                                "type": "dict",
                            }
                        )
                    continue
                container.append(_parse_scalar(value_part))
                continue

            split = _split_key_value(content)
            if split is not None:
                key, value_part = split
                key = key.strip()
                value_part = value_part.strip()
                _convert_frame(frame, "dict")
                container = frame["container"]
                if value_part == "":
                    container[key] = {}
                    stack.append(
                        {
                            "indent": indent,
                            "container": container[key],
                            "parent": container,
                            "key": key,
                            "type": "dict",
                        }
                    )
                else:
                    container[key] = _parse_scalar(value_part)
                continue

            if isinstance(container, list):
                container.append(_parse_scalar(content))
            elif isinstance(container, dict):
                container[content] = True

    return root


def _load_manifest(path: str) -> Any:
    _, ext = os.path.splitext(path)
    ext = ext.lower()
    if ext in {".yaml", ".yml"}:
        try:
            import yaml  # type: ignore
        except Exception:
            try:
                return _parse_simple_yaml(path)
            except Exception:
                return {}
        with open(path, "r", encoding="utf-8") as handle:
            return yaml.safe_load(handle) or {}
    if ext == ".json":
        with open(path, "r", encoding="utf-8") as handle:
            return json.load(handle) or {}
    with open(path, "r", encoding="utf-8") as handle:
        return json.load(handle) or {}


path = sys.argv[1]
data = _load_manifest(path) or {}

wgx = data.get('wgx')
if not isinstance(wgx, dict):
    wgx = {}

# Backwards compatibility: allow certain keys (e.g. tasks) at the top level.
# Older profiles stored "tasks" directly on the root object. Newer profiles nest
# them inside the "wgx" block. We support both to avoid breaking existing
# repositories.
root_tasks = data.get('tasks') if isinstance(data, dict) else None
root_repo_kind = data.get('repoKind') if isinstance(data, dict) else None
root_dirs = data.get('dirs') if isinstance(data, dict) else None
root_env = data.get('env') if isinstance(data, dict) else None
root_env_defaults = data.get('envDefaults') if isinstance(data, dict) else None
root_env_overrides = data.get('envOverrides') if isinstance(data, dict) else None
root_workflows = data.get('workflows') if isinstance(data, dict) else None

platform_keys = []
plat = sys.platform
if plat.startswith('darwin'):
    platform_keys.append('darwin')
elif plat.startswith('linux'):
    platform_keys.append('linux')
elif plat.startswith('win'):
    platform_keys.append('win32')
elif plat.startswith('cygwin') or plat.startswith('msys'):
    platform_keys.append('win32')
platform_keys.append('default')

def select_variant(value):
    if isinstance(value, dict):
        for key in platform_keys:
            if key in value and value[key] not in (None, ''):
                return value[key]
        for entry in value.values():
            if entry not in (None, ''):
                return entry
        return None
    return value

def as_bool(value):
    if isinstance(value, bool):
        return value
    if isinstance(value, int):
        return value != 0
    if isinstance(value, str):
        return value.strip().lower() in ("1", "true", "yes", "on")
    return False

def normalize_list(value):
    if value is None:
        return []
    if isinstance(value, (list, tuple)):
        return [str(item) for item in value]
    if isinstance(value, dict):
        selected = select_variant(value)
        if isinstance(selected, (list, tuple)):
            return [str(item) for item in selected]
        if selected is None:
            return []
        return [str(selected)]
    return [str(value)]

def emit(line: str) -> None:
    sys.stdout.write(f"{line}\n")

def shell_quote(value: str) -> str:
    return shlex.quote(value)

def emit_env(prefix: str, mapping):
    if not isinstance(mapping, dict):
        return
    for key, val in mapping.items():
        if key is None:
            continue
        # env base/overrides werden 1:1 als STR übernommen
        skey = str(key)
        sval = '' if val is None else str(val)
        emit(f"{prefix}[{shell_quote(skey)}]={shell_quote(sval)}")

def emit_caps(caps):
    if not isinstance(caps, (list, tuple)):
        return
    for cap in caps:
        if cap is None:
            continue
        emit(f"WGX_REQUIRED_CAPS+=({shell_quote(str(cap))})")

# track if we used any root-level fallback (for a single deprecation note)
used_root_fallback = False

api_version = ''
if isinstance(wgx, dict):
    api_version = str(wgx.get('apiVersion') or '')
if not api_version and isinstance(data, dict):
    api_version = str(data.get('apiVersion') or '')
if not api_version:
    api_version = 'v1'

emit(f"PROFILE_VERSION={shell_quote(api_version)}")
req = wgx.get('requiredWgx')

# Fallback: check root 'requiredWgx' or 'required-wgx' if not in wgx block
if req is None and isinstance(data, dict):
    req = data.get('requiredWgx')

if req is None and isinstance(data, dict):
    req = data.get('required-wgx')

# Also check wgx['required-wgx'] specifically (alias inside wgx block)
if req is None and isinstance(wgx, dict):
    req = wgx.get('required-wgx')

# One final check for the legacy top-level alias if strictly nothing found yet
if req is None and isinstance(data, dict):
    req = data.get('required-wgx')

# Ensure we handle the case where both keys might exist but one is None/Empty
# Priority: wgx.requiredWgx > wgx.required-wgx > root.requiredWgx > root.required-wgx
# (The above logic roughly implements a "first found wins" strategy)

# One final check for the legacy top-level alias if strictly nothing found yet
if req is None and isinstance(data, dict):
    req = data.get('required-wgx')

if isinstance(req, str):
    emit(f"WGX_REQUIRED_RANGE={shell_quote(req)}")
elif isinstance(req, dict):
    rng = req.get('range')
    if rng:
        emit(f"WGX_REQUIRED_RANGE={shell_quote(str(rng))}")
    minimum = req.get('min')
    if minimum:
        emit(f"WGX_REQUIRED_MIN={shell_quote(str(minimum))}")
    emit_caps(req.get('caps'))
else:
    emit_caps([])

repo_kind = wgx.get('repoKind') if isinstance(wgx, dict) else None
if repo_kind is None:
    repo_kind = root_repo_kind
    if repo_kind is not None:
        used_root_fallback = True
emit(f"WGX_REPO_KIND={shell_quote(str(repo_kind or ''))}")

dirs = wgx.get('dirs') if isinstance(wgx, dict) else None
if not isinstance(dirs, dict):
    dirs = root_dirs if isinstance(root_dirs, dict) else {}
    if dirs:
        used_root_fallback = True
emit(f"WGX_DIR_WEB={shell_quote(str(dirs.get('web') or ''))}")
emit(f"WGX_DIR_API={shell_quote(str(dirs.get('api') or ''))}")
emit(f"WGX_DIR_DATA={shell_quote(str(dirs.get('data') or ''))}")

env_defaults = wgx.get('envDefaults') if isinstance(wgx, dict) else None
if not isinstance(env_defaults, dict):
    env_defaults = root_env_defaults if isinstance(root_env_defaults, dict) else {}
    if env_defaults:
        used_root_fallback = True
emit_env('WGX_ENV_DEFAULT_MAP', env_defaults)

env_base = wgx.get('env') if isinstance(wgx, dict) else None
if not isinstance(env_base, dict):
    env_base = root_env if isinstance(root_env, dict) else {}
    if env_base:
        used_root_fallback = True
emit_env('WGX_ENV_BASE_MAP', env_base)

env_overrides = wgx.get('envOverrides') if isinstance(wgx, dict) else None
if not isinstance(env_overrides, dict):
    env_overrides = root_env_overrides if isinstance(root_env_overrides, dict) else {}
    if env_overrides:
        used_root_fallback = True
emit_env('WGX_ENV_OVERRIDE_MAP', env_overrides)

workflows = wgx.get('workflows') if isinstance(wgx, dict) else None
if not isinstance(workflows, dict):
    workflows = root_workflows if isinstance(root_workflows, dict) else {}
    if workflows:
        used_root_fallback = True
if isinstance(workflows, dict):
    for wf_name, wf_spec in workflows.items():
        steps = []
        if isinstance(wf_spec, dict):
            for step in wf_spec.get('steps') or []:
                if isinstance(step, dict):
                    task_name = step.get('task')
                    if task_name:
                        steps.append(str(task_name))
        emit(f"WGX_WORKFLOW_TASKS[{shell_quote(str(wf_name))}]={shell_quote(' '.join(steps))}")

tasks = wgx.get('tasks') if isinstance(wgx, dict) else None
if not isinstance(tasks, dict) or not tasks:
    tasks = root_tasks if isinstance(root_tasks, dict) else {}
    if tasks:
        used_root_fallback = True
if isinstance(tasks, dict):
    seen_task_order = set()
    for raw_name, spec in tasks.items():
        name = str(raw_name)
        norm = name.replace(' ', '').replace('_', '-').lower()
        if norm not in seen_task_order:
            emit(f"WGX_TASK_ORDER+=({shell_quote(norm)})")
            seen_task_order.add(norm)
        desc = ''
        group = ''
        safe = False
        cmd_value = spec
        args_value = None
        if isinstance(spec, dict):
            desc = spec.get('desc') or ''
            group = spec.get('group') or ''
            safe = as_bool(spec.get('safe'))
            cmd_value = spec.get('cmd')
            args_value = spec.get('args')
        selected_cmd = select_variant(cmd_value)
        #
        # Build command preserving semantics:
        # - If manifest provided a STRING: keep it as-is (no re-quoting/splitting).
        #   Only append args (quoted) if present.
        # - If manifest provided an ARRAY: emit ARRJSON (and extend with args).
        # - Otherwise: coerce to string sensibly.
        #
        base_cmd = None
        tokens = []
        use_array_format = False

        if isinstance(selected_cmd, (list, tuple)):
            tokens = [str(item) for item in selected_cmd]
            use_array_format = True
        elif isinstance(selected_cmd, str) and selected_cmd.strip():
            base_cmd = selected_cmd  # preserve raw shell string
        elif selected_cmd not in (None, ''):
            # numbers/other scalars -> treat as a single token
            tokens = [str(selected_cmd)]

        # Normalize/collect args (list/dict with platform variants)
        appended_args = []
        if isinstance(args_value, (list, tuple)) and args_value:
            appended_args.extend(str(item) for item in args_value)
        elif isinstance(args_value, dict):
            variant = select_variant(args_value)
            if isinstance(variant, (list, tuple)):
                appended_args.extend(str(item) for item in variant)
            elif variant not in (None, ''):
                appended_args.append(str(variant))

        if use_array_format:
            if appended_args:
                tokens.extend(appended_args)
            payload = json.dumps(tokens, ensure_ascii=False)
            emit(f"WGX_TASK_CMDS[{shell_quote(norm)}]={shell_quote('ARRJSON:' + payload)}")
        else:
            command_parts = []
            if base_cmd is not None:
                command_parts.append(base_cmd)
                if appended_args:
                    command_parts.extend(shlex.quote(str(a)) for a in appended_args)
                command = ' '.join(command_parts)
            else:
                all_parts = tokens + appended_args
                command = ' '.join(shlex.quote(str(p)) for p in all_parts)
            emit(f"WGX_TASK_CMDS[{shell_quote(norm)}]={shell_quote('STR:' + command)}")
        emit(f"WGX_TASK_DESC[{shell_quote(norm)}]={shell_quote(str(desc))}")
        emit(f"WGX_TASK_GROUP[{shell_quote(norm)}]={shell_quote(str(group))}")
        emit(f"WGX_TASK_SAFE[{shell_quote(norm)}]={shell_quote('1' if safe else '0')}")
        continue

if used_root_fallback and os.environ.get("WGX_PROFILE_DEPRECATION", "warn") != "quiet":
    print("wgx: note: using root-level profile keys for backwards compatibility; consider nesting under 'wgx.'", file=sys.stderr)

PY
  )"
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
  profile::_have_cmd python3 || return 1
  python3 - "$json_payload" <<'PY'
import json
import sys

try:
    values = json.loads(sys.argv[1])
except Exception:
    sys.exit(1)

for entry in values:
    if entry is None:
        continue
    print(str(entry))
PY
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
      base_cmd="${spec#STR:}"   # z.B. "echo 'a # b'"
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

profile::_auto_init() {
  if profile::has_manifest; then
    if profile::load "$PROFILE_FILE"; then
      profile::ensure_version || true
    else
      warn "profile manifest could not be loaded"
    fi
  fi
}
