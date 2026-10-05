from pathlib import Path
import sys
root=Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)
for multiplicity in (2,10,40,63,64,65,100,1000):
 p=root/f'multiplicity-{multiplicity}';p.mkdir(exist_ok=True)
 source='function f(x: number) { '+' '.join(f'x += {i};' for i in range(40))+' return x; }\n'
 for i in range(multiplicity): (p/f'f{i:04}.ts').write_text(source)
p=root/'changed-100k';p.mkdir(exist_ok=True)
for i in range(20):
 source=f'function f{i}(x: number) {{ '+' '.join(f'x += {i*10000+j};' for j in range(1250))+' return x; }\n'
 (p/f'f{i:04}.ts').write_text(source)
