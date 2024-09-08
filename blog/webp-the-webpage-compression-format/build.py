import base64
import subprocess

subprocess.run(["yarn", "build"], check=True)

with open("index.html") as f:
    html = f.read()
with open("nojs.html", "w") as f:
    f.write(html)

before = html.partition("<cut></cut>")[0]

proc = subprocess.run("compressor/target/release/compressor", input=html.encode(), capture_output=True, check=True)
webp = proc.stdout
width, height = map(int, proc.stderr.decode().strip().split('x'))

url = "data:image/webp;base64," + base64.b64encode(webp).decode()

s = before + f"""<noscript><meta http-equiv=refresh content=0;url=nojs.html></noscript><div style=height:100000px><script type=module>try{{let c=new OffscreenCanvas({width},{height}).getContext("webgl"),t=c.createTexture(),p=new Uint8Array({width * height * 4}),y=new Uint8Array({len(html.encode())}),i=0
c.bindTexture(3553,t)
c.texImage2D(3553,0,6408,6408,5121,await createImageBitmap(await (await fetch`{url}`).blob()))
c.bindFramebuffer(36160,c.createFramebuffer())
c.framebufferTexture2D(36160,36064,3553,t,0)
c.readPixels(0,0,{width},{height},6408,5121,p)
for(;i<y.length;i++)y[i]=p[i*4]
document.documentElement.innerHTML=new TextDecoder().decode(y)}}catch(e){{location.href="nojs.html"}}
</script>"""

with open("index.html", "w") as f:
    f.write(s)
