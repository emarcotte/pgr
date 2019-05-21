# PGR

Mostly an excuse to learn rust.

Print's out a process tree. By default prints current users processes. The only two options are:

1. `-a` to show processes for all users.
2. a single string used as a simple filter to process names. Any matching process and its children are printed.

It will wrap long command names which isn't useful for `grep`ing but is useful for humans.

Example:

```
[emarcotte@emarcotte-x1new pgr]$ target/release/pgr | head
├─ 2147 /usr/libexec/gdm-x-session --run-script /usr/bin/gnome-session 
│  ├─ 2149 /usr/libexec/Xorg vt2 -displayfd 3 -auth /run/user/1000/gdm/Xauthority -background none -noreset -keeptty -verbose 3 
│  └─ 2301 /usr/libexec/gnome-session-binary 
│     ├─ 2322 /usr/bin/ssh-agent /bin/sh -c "exec -l /bin/bash -c "/usr/bin/gnome-session"" 
│     ├─ 2422 /usr/bin/gnome-shell 
│     │  ├─ 2351 /usr/lib64/firefox/firefox 
│     │  │  ├─ 2508 /usr/lib64/firefox/firefox -contentproc -childID 1 -isForBrowser -prefsLen 1 ....
│     │  │  ├─ 2709 /usr/lib64/firefox/firefox -contentproc -childID 2 -isForBrowser -prefsLen 6011 ....
│     │  │  ├─ 2808 /usr/bin/python3 /usr/bin/chrome-gnome-shell /usr/lib64/mozilla/native-messaging-hosts/org.gnome.chrome_gnome_shell.json chrome-gnome-shell@gnome.org 
│     │  │  ├─ 6004 /usr/lib64/firefox/firefox -contentproc -childID 83 -isForBrowser -prefsLen 11085 ...
```