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
