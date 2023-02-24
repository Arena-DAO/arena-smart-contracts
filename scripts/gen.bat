@echo off

REM generate all schema
for /d %%f in (..\contracts\*) do (
  cd "%%f"
  echo generating schema for %%f
  cargo run schema > NUL
  rd /s /q .\schema\raw
  cd ..
)
REM create typescript types
cd ..\ts-codegen
call pnpm i && pnpm run gen