set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "$script_path/.."

CRATE_NAME="egui_demo_app"
FEATURES="http,persistence,screen_reader"

OPEN=false
FAST=false

while test $# -gt 0; do
  case "$1" in
    -h|--help)
      echo "build_demo_web.sh [--fast] [--open]"
      echo "  --fast: skip optimization step"
      echo "  --open: open the result in a browser"
      exit 0
      ;;
    --fast)
      shift
      FAST=true
      ;;
    --open)
      shift
      OPEN=true
      ;;
    *)
      break
      ;;
  esac
done
