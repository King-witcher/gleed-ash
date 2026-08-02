#!/usr/bin/env bash

GREEN="\e[1;32m"
RED="\e[0;31m"
PURPLE="\e[0;35m"
NC="\e[0m"

# Remove existing .spv files
# spvs=$(find . -name '*.spv')
# for spv in $spvs; do
#   rm "$spv"
# done

function compile {
  local file_path="$1"

  local dir=$(dirname "$file_path")
  local filename=$(basename "$file_path")
  local basename=${filename%.*}

  slangc "$file_path"          \
    -target spirv              \
    -profile spirv_1_4         \
    -emit-spirv-directly       \
    -fvk-use-entrypoint-name   \
    -entry vertMain            \
    -entry fragMain            \
    -o "$dir/${basename}.spv"
}

function out_path {
  local dir=$(dirname "$1")
  local filename=$(basename "$1")
  echo "$dir/${filename%.*}.spv"
}

echo -e "${PURPLE}SLANGC: compiling...${NC}"

# Collect shaders (NUL-delimited, so paths with spaces survive)
shaders=()
while IFS= read -r -d '' shader; do
  shaders+=("$shader")
done < <(find ./src -name '*.slang' -print0)

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT

# Fan out: one background job per shader
pids=()
logs=()
for i in "${!shaders[@]}"; do
  logs[i]="$tmpdir/$i.log"
  compile "${shaders[i]}" >"${logs[i]}" 2>&1 &
  pids[i]=$!
done

# Await all, then report
failures=0
for i in "${!pids[@]}"; do
  if wait "${pids[i]}"; then
    echo -e "${GREEN}[SUCCESS]${NC} ${shaders[i]} -> ${PURPLE}$(out_path "${shaders[i]}")${NC}"
  else
    echo -e "${RED}[FAILED]${NC} ${shaders[i]}${NC}"
    failures=$((failures + 1))
  fi
  # Compiler diagnostics (warnings on success, errors on failure)
  [ -s "${logs[i]}" ] && cat "${logs[i]}"
done

if [ "$failures" -ne 0 ]; then
  echo -e "${RED}SLANGC: $failures shader(s) failed${NC}"
  exit 1
fi

echo -e "${GREEN}SLANGC: ${#shaders[@]} shader(s) compiled${NC}"
