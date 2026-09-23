from PIL import Image, ImageDraw


def logo(size=(192, 128)):
    image = Image.new("RGBA", size, (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    draw.rounded_rectangle((16, 16, 112, 112), radius=24, fill=(32, 105, 210, 255))
    draw.ellipse((48, 48, 80, 80), fill=(0, 0, 0, 0))
    draw.polygon([(136, 24), (176, 64), (136, 104)], fill=(240, 128, 48, 128))
    return image


def line_art():
    image = Image.new("RGB", (128, 96), "white")
    draw = ImageDraw.Draw(image)
    draw.line([(12, 60), (40, 18), (72, 60), (108, 18)], fill="black", width=2)
    draw.line((12, 80, 108, 80), fill="black", width=1)
    draw.point((120, 8), fill="black")
    return image


def illustration():
    image = Image.new("RGB", (128, 96), (230, 240, 255))
    draw = ImageDraw.Draw(image)
    for i in range(16):
        draw.rectangle((8 * i, 48, 8 * i + 7, 95), fill=(20 + i * 12, 130, 190 - i * 8))
    draw.ellipse((14, 8, 42, 36), fill=(255, 190, 40))
    draw.rectangle((90, 12, 92, 14), fill=(190, 30, 60))
    return image
