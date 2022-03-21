ECHO OFF
CLS

SET ROOT_FOLDER="C:\Users\denis\wks-one\target\debug"

REM echo
REM echo ******************
REM echo ***** ROBOT ****
REM echo ******************
REM start "robot-server" java -jar %ROOT_FOLDER%\robot\robot-server\target\robot-server-1.3-fillim-all.jar


echo *************************
echo ***** KEY MANAGER *******
echo *************************
start "key-manager"  %ROOT_FOLDER%\key-manager.exe

echo **************************
echo ***** SESSION MANAGER ****
echo **************************
start "session-manager" %ROOT_FOLDER%\session-manager.exe

echo **************************
echo ***** ADMIN SERVER *******
echo **************************
start "admin-server" %ROOT_FOLDER%\admin-server.exe

echo *****************************
echo ***** DOCUMENT SERVER *******
echo *****************************
start "document-server" %ROOT_FOLDER%\document-server.exe

echo *****************************
echo ***** FILE SERVER *******
echo *****************************
start "file-server" %ROOT_FOLDER%\file-server.exe

echo *****************************
echo ***** TIKA SERVER *******
echo *****************************
start "tika-server" java -jar c:\Users\denis\wks-poc\tika\tika-server-standard-2.2.0.jar --port 40010

