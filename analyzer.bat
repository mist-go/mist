@echo off

cd D:\projects\mist\mist

mist build >nul 2>&1

target\debug\mist-analyzer.exe
