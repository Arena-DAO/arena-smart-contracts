START_DIR=$(pwd)
# ${f    <-- from variable f
#   ##   <-- greedy front trim
#   *    <-- matches anything
#   /    <-- until the last '/'
#  }
# <https://stackoverflow.com/a/3162500>
cd "$START_DIR"
for f in ./contracts/*
do
  echo "generating schema for ${f##*/}"
  cd "$f"
  CMD="cargo run schema"
  eval $CMD > /dev/null
  cd "$START_DIR"
done