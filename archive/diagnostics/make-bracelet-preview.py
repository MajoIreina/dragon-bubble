from pathlib import Path
from PIL import Image, ImageDraw, ImageFont
root=Path('L:/LPP')
sheet=Image.new('RGB',(1500,700),'#edf2ed')
draw=ImageDraw.Draw(sheet)
font=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',26)
small=ImageFont.truetype('C:/Windows/Fonts/msyh.ttc',19)
draw.text((36,23),'黍泡泡 · 珠串与面部修正',font=font,fill='#30483b')
draw.text((36,64),'珠串沿角根和头部斜坡环绕 · 正脸竖线移除',font=small,fill='#6f8174')
before=Image.open(root/'work/shu-bracelet-before-front.png').convert('RGBA')
after=Image.open(root/'outputs/shu-front.png').convert('RGBA')
boxes=[before.getbbox(),after.getbbox()]
box=(min(b[0] for b in boxes)-22,min(b[1] for b in boxes)-22,max(b[2] for b in boxes)+22,max(b[3] for b in boxes)+22)
images=[before.crop(box),after.crop(box),Image.open(root/'outputs/shu-bracelet-front-detail.png').crop((270,330,850,880)).convert('RGBA')]
for i,(image,label) in enumerate(zip(images,['修正前','修正后','珠串近照'])):
    x=18+i*494
    draw.rounded_rectangle((x,109,x+476,676),radius=22,fill='#f8faf5')
    image.thumbnail((450,465),Image.Resampling.LANCZOS)
    sheet.paste(image,(x+(476-image.width)//2,132+(465-image.height)//2),image)
    draw.text((x+(476-draw.textlength(label,font=small))//2,631),label,font=small,fill='#536d5b')
sheet.save(root/'outputs/shu-bracelet-fix-review.png')
print(root/'outputs/shu-bracelet-fix-review.png')
