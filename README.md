# PGR

Mostly an excuse to learn rust.

Print's out a process tree. By default prints current users processes. The only two options are:

1. `-a` to show processes for all users.
2. a single string used as a simple filter to process names. Any matching process and its children are printed.

It will wrap long command names which isn't useful for `grep`ing but is useful for humans.

Example:

```
[emarcotte@emarcotte-x1new pgr]$ pgr tmux            
├─ 3620 tmux                                         
│  ├─ 4066 -bash                                     
│  │  └─ 5183 /home/emarcotte/.cargo/bin/cargo-watch watch -x test -x run                                 
│  ├─ 4101 -bash                                     
│  │  └─ 4894 vim                                    
│  │     └─ 4909 node --no-warnings                  
│  │        │     /home/emarcotte/Projects/dotfiles/files/.vim/plugged/coc.nvim/bin/server.js             
│  │        └─ 5003 /home/emarcotte/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rls            
│  ├─ 10062 -bash                                    
│  │  └─ 16417 /home/emarcotte/.cargo/bin/cargo-watch watch -x build -x test                              
│  ├─ 14552 -bash                                    
│  │  └─ 4645 vim src/main.rs                        
│  │     └─ 4646 node --no-warnings                  
│  │        │     /home/emarcotte/Projects/dotfiles/files/.vim/plugged/coc.nvim/bin/server.js             
│  │        └─ 5000 /home/emarcotte/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rls            
│  ├─ 29455 -bash                                    
│  │  └─ 26479 pgr tmux                              
│  └─ 32706 -bash                                    
└─ 14884 tmux at   
```