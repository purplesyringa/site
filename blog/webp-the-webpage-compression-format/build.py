import base64
import subprocess

subprocess.run(["yarn", "build"], check=True)

with open("index.html") as f:
    html = f.read()
with open("nojs.html", "w") as f:
    f.write(html)

before = html.partition("<cut></cut>")[0]

webp = subprocess.run("compressor/target/release/compressor", input=html.encode(), capture_output=True, check=True).stdout

url = "data:image/webp;base64," + base64.b64encode(webp).decode()

s = before + f"""<noscript><meta http-equiv=refresh content=0;url=nojs.html></noscript><div style=height:100000px><script type=module>var b=await createImageBitmap(await(await fetch("{url}")).blob()),c=new OffscreenCanvas(b.width,b.height).getContext("2d"),p,y,i;
c.drawImage(b,0,0)
p=c.getImageData(0,0,b.width,b.height).data
y=new Uint8Array(p.length/4)
for(i=0;i<y.length;i++)y[i]=p[i*4]
document.documentElement.innerHTML=new TextDecoder().decode(y)</script>"""

with open("index.html", "w") as f:
    f.write(s)
