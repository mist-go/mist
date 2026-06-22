@echo off

cd D:\projects\mist\mist

mist build >nul 2>&1

cd mist-test

D:\projects\mist\mist\target\debug\mist-analyzer.exe
