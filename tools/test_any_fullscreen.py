"""Real, unfocused Win32 windows in a separate process; never activate them."""
import argparse
import ctypes as c
from ctypes import wintypes as w
import subprocess
import sys

def fixture():
    user=c.WinDLL('user32',use_last_error=True)
    user.CreateWindowExW.restype=w.HWND
    user.CreateWindowExW.argtypes=[w.DWORD,w.LPCWSTR,w.LPCWSTR,w.DWORD,c.c_int,c.c_int,c.c_int,c.c_int,w.HWND,w.HMENU,w.HINSTANCE,c.c_void_p]
    user.SetWindowPos.argtypes=[w.HWND,w.HWND,c.c_int,c.c_int,c.c_int,c.c_int,w.UINT]
    user.ShowWindow.argtypes=[w.HWND,c.c_int]
    user.DestroyWindow.argtypes=[w.HWND]
    user.GetForegroundWindow.restype=w.HWND
    user.GetMonitorInfoW.argtypes=[w.HANDLE,c.c_void_p]
    user.MonitorFromWindow.argtypes=[w.HWND,w.DWORD];user.MonitorFromWindow.restype=w.HANDLE
    class INFO(c.Structure): _fields_=[('size',w.DWORD),('monitor',w.RECT),('work',w.RECT),('flags',w.DWORD)]
    info=INFO();info.size=c.sizeof(info)
    assert user.GetMonitorInfoW(user.MonitorFromWindow(None,1),c.byref(info))
    r=info.monitor
    window=user.CreateWindowExW(0,'STATIC','Dragon Bubble fullscreen fixture',0x80000000,r.left,r.top,r.right-r.left,r.bottom-r.top,None,None,None,None)
    assert window
    foreground=user.GetForegroundWindow()
    print('READY',flush=True)
    try:
        for line in sys.stdin:
            command=line.strip()
            if command=='full':
                user.SetWindowPos(window,None,r.left,r.top,r.right-r.left,r.bottom-r.top,0x14)
                user.ShowWindow(window,8)
            elif command=='windowed':
                user.SetWindowPos(window,None,r.left+40,r.top+40,640,400,0x14)
            elif command=='minimize': user.ShowWindow(window,6)
            elif command=='hide': user.ShowWindow(window,0)
            elif command=='quit': break
            c.WinDLL('dwmapi').DwmFlush()
            assert user.GetForegroundWindow()==foreground,'Fixture must remain unfocused'
            print('OK',flush=True)
    finally: user.DestroyWindow(window)

def main():
    parser=argparse.ArgumentParser();parser.add_argument('exe');args=parser.parse_args()
    child=subprocess.Popen([sys.executable,__file__,'--fixture'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
    try:
        assert child.stdout.readline().strip()=='READY'
        for command,expected in [('full',True),('windowed',False),('full',True),('minimize',False),('hide',False)]:
            child.stdin.write(command+'\n');child.stdin.flush();assert child.stdout.readline().strip()=='OK'
            run=subprocess.run([args.exe,'--fullscreen-status'],capture_output=True,text=True,timeout=10)
            assert run.returncode==0 and ('FULLSCREEN STATUS: '+str(expected).lower()) in run.stdout,(command,run.stdout,run.stderr)
        print('ANY FULLSCREEN PASS: unfocused fullscreen detected; resize, minimize and hide restore visibility')
    finally:
        child.stdin.write('quit\n');child.stdin.flush();child.wait(timeout=10)

if __name__=='__main__':
    if '--fixture' in sys.argv: fixture()
    else: main()
