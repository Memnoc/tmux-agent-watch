"""Small VT cell observer for attached-client UI tests (never used by runtime)."""
import re
import unicodedata

class TerminalCells:
    def __init__(self, width, height=40):
        self.width, self.height = width, height
        self.grid = [[' '] * width for _ in range(height)]
        self.styles = [[()] * width for _ in range(height)]
        self.x = self.y = 0
        self.style = (None,None,False)
        self.saved = (0, 0)

    def feed(self, data):
        text = data.decode('utf8', errors='replace') if isinstance(data, bytes) else data
        i = 0
        while i < len(text):
            c = text[i]; i += 1
            if c == '\x1b':
                if i >= len(text): break
                kind = text[i]; i += 1
                if kind == '[':
                    start = i
                    while i < len(text) and not '@' <= text[i] <= '~': i += 1
                    if i == len(text): break
                    raw, final = text[start:i], text[i]; i += 1
                    values = [int(v) if v.isdigit() else 0 for v in raw.split(';')]
                    n = values[0] or 1
                    if final in ('H','f'):
                        self.y = min(self.height-1, n-1)
                        self.x = min(self.width-1, (values[1] or 1)-1 if len(values)>1 else 0)
                    elif final == 'd': self.y = min(self.height-1,n-1)
                    elif final in ('G','`'): self.x = min(self.width-1,n-1)
                    elif final == 'A': self.y = max(0,self.y-n)
                    elif final in ('B','e'): self.y = min(self.height-1,self.y+n)
                    elif final in ('C','a'): self.x = min(self.width-1,self.x+n)
                    elif final == 'D': self.x = max(0,self.x-n)
                    elif final == 'K':
                        lo,hi = (0,self.width) if values[0]==2 else (0,self.x+1) if values[0]==1 else (self.x,self.width)
                        for x in range(lo,hi): self.grid[self.y][x]=' ';self.styles[self.y][x]=self.style
                    elif final == 'J':
                        for y in range(self.height):
                            for x in range(self.width):
                                if values[0]==2 or (y,x)>=(self.y,self.x):self.grid[y][x]=' ';self.styles[y][x]=self.style
                    elif final == 'm':
                        fg,bg,bold=self.style
                        n=0
                        while n<len(values):
                            v=values[n];n+=1
                            if v==0:fg=bg=None;bold=False
                            elif v==1:bold=True
                            elif v==22:bold=False
                            elif v==39:fg=None
                            elif v==49:bg=None
                            elif 30<=v<=37:fg=('index',v-30)
                            elif 40<=v<=47:bg=('index',v-40)
                            elif 90<=v<=97:fg=('index',v-90+8)
                            elif 100<=v<=107:bg=('index',v-100+8)
                            elif v in (38,48) and n<len(values):
                                mode=values[n];n+=1
                                if mode==5 and n<len(values):color=('index',values[n]);n+=1
                                elif mode==2 and n+2<len(values):color=('rgb',*values[n:n+3]);n+=3
                                else:continue
                                if v==38:fg=color
                                else:bg=color
                        self.style=(fg,bg,bold)
                    elif final == 's': self.saved=(self.x,self.y)
                    elif final == 'u': self.x,self.y=self.saved
                elif kind in (']','P','_'):
                    end = re.search(r'\x07|\x1b\\',text[i:])
                    i = i+end.end() if end else len(text)
                elif kind in ('(',')','*','+'):
                    i += 1
                elif kind == '7': self.saved=(self.x,self.y)
                elif kind == '8': self.x,self.y=self.saved
                continue
            if c == '\r': self.x=0
            elif c == '\n': self.y=min(self.height-1,self.y+1)
            elif c == '\b': self.x=max(0,self.x-1)
            elif c == '\t': self.x=min(self.width-1,(self.x//8+1)*8)
            elif ord(c)>=32:
                if unicodedata.combining(c):
                    if self.x:self.grid[self.y][self.x-1]+=c
                    continue
                size=2 if unicodedata.east_asian_width(c) in ('W','F') else 1
                if self.x>=self.width:self.x=0;self.y=min(self.height-1,self.y+1)
                self.grid[self.y][self.x]=c;self.styles[self.y][self.x]=self.style
                if size==2 and self.x+1<self.width:self.grid[self.y][self.x+1]='';self.styles[self.y][self.x+1]=self.style
                self.x += size
        return self

    def lines(self): return [''.join(row) for row in self.grid]
