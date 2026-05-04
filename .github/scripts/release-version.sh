normalize_version() {
  local tag="${1#v}"
  local core="${tag%%[-+]*}"

  if [[ ! "${core}" =~ ^[0-9]+\.[0-9]+(\.[0-9]+)?$ ]]; then
    return 1
  fi

  local major minor patch
  IFS=. read -r major minor patch <<< "${core}"
  printf '%s.%s.%s\n' "${major}" "${minor}" "${patch:-0}"
}

version_ge() {
  local version minimum
  version="$(normalize_version "${1}")" || return 1
  minimum="$(normalize_version "${2}")" || return 1
  [[ "$(printf '%s\n%s\n' "${minimum}" "${version}" | sort -V | head -n 1)" == "${minimum}" ]]
}

extract_version() {
  printf '%s\n' "${1}" | grep -Eo '[0-9]+(\.[0-9]+){1,2}' | head -n 1 || true
}
