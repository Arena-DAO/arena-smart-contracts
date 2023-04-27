@echo off

REM generate all schema
for /d %%f in (.\contracts\*) do (
  echo generating schema for %%f
  cd "%%f"
  cargo schema > NUL
  rd /s /q .\schema\raw > NUL
  cd ..
  cd ..
)
REM create typescript types
cd .\ts-codegen
call pnpm i && pnpm run gen