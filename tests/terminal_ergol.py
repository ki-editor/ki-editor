"""Linux PTY regression for the Kitty sequences captured from patched Ghostty.

Run after `cargo build`: python3 tests/terminal_ergol.py [path/to/ki]
Uses an isolated configuration and temporary buffer; does not inject desktop keys.
"""
import os, pty, select, time, tempfile, fcntl, termios, struct, signal, sys, json
from pathlib import Path
binary = Path(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/ki').resolve()
root=Path(tempfile.mkdtemp(prefix='ki-ergol-'))
(root / '.ki').mkdir()
(root / '.ki/config.json').write_text(json.dumps({
 'keyboard_layout': 'ERGOL',
 'custom_keyboard_layouts': {'ERGOL': [list('qcopwjmd★y'), list('asenflrtiu'), list('zx-vb.hg,k')]}
}))
file=root/'typed.txt'; file.write_text('')
pid, fd=pty.fork()
if pid==0:
 os.environ['XDG_CONFIG_HOME']=str(root/'config')
 os.environ['XDG_STATE_HOME']=str(root/'state')
 os.environ['XDG_CACHE_HOME']=str(root/'cache')
 os.environ['TERM']='xterm-ghostty'
 os.chdir(root)
 os.execl(str(binary), 'ki', str(file))
fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack('HHHH',40,120,0,0))
output=bytearray()
def drain(seconds):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  if select.select([fd],[],[],.05)[0]:
   try:data=os.read(fd,65536)
   except OSError:return
   output.extend(data)
   if b'\x1b[c' in data:os.write(fd,b'\x1b[?62;22c')
   if b'\x1b[?u' in data:os.write(fd,b'\x1b[?31u')
def send(seq):
 os.write(fd,seq);drain(.15)
try:
 drain(2)
 assert b'\x1b[>31u' in output, bytes(output[-1000:])
 # Ergo-L l is physical H: enter insert mode positionally.
 send(b'\x1b[108::104;;108u');send(b'\x1b[108::104;1:3u')
 send(b'\x1b[57454::111u');send(b'\x1b[57454::111;1:3u')
 send(b'\x1b[97;;224u');send(b'\x1b[97;1:3u')
 send(b'\x1b[99::119;;99u');send(b'\x1b[99::119;1:3u')
 send(b'\x1b[97;1:2;97:776u');send(b'\x1b[97;1:3u')
 send(b'\x1b[27u');send(b'\x1b[27;1:3u');send(b'\x1b[13u');send(b'\x1b[13;1:3u')
 drain(1)
 actual=file.read_text()
 assert actual=='àca\u0308',repr(actual)
 print('PASS: flags31, physical H command, silent star, composed text, repeat and releases; saved',repr(actual))
finally:
 (root/'terminal-output').write_bytes(output)
 print('Artifacts:',root)
 os.kill(pid,signal.SIGTERM);os.waitpid(pid,0);os.close(fd)
