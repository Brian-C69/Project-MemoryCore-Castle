@echo off
rem castle-memory: the companion's door to the memory database.
rem Starts Castle's own program with --memory; it answers and ends, no window.
rem Long text goes through --file <path>, never on the command line.
if "%CASTLE_EXE%"=="" (
  echo castle-memory: run this from a session Castle started ^(CASTLE_EXE is not set^). 1>&2
  exit /b 1
)
"%CASTLE_EXE%" --memory %*
