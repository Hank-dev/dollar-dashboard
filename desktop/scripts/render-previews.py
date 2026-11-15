#!/usr/bin/python
"""Render exported egui meshes for visual QA without a running compositor.

Run cargo test export_ui_previews -- --ignored first.
Uses system NumPy/Pillow; neither is a runtime dependency of the app.
"""
from pathlib import Path
import json
import numpy as np
from PIL import Image


def render(path):
    data = json.loads(path.read_text())
    width, height = map(int, data['size'])
    target = np.empty((height, width, 4), dtype=np.float32)
    target[:] = [0, 0, 0, 1]
    textures = {}
    for texture in data['textures']:
        tw, th = texture['size']
        textures[texture['id']] = np.fromfile(path.parent/texture['file'], dtype=np.uint8).reshape(th, tw, 4).astype(np.float32)/255
    for mesh in data['meshes']:
        vertices = np.array(mesh['vertices'], dtype=np.float32)
        texture = textures[mesh['texture']]
        th, tw = texture.shape[:2]
        clip = mesh['clip']
        for indices in np.array(mesh['indices']).reshape(-1, 3):
            tri = vertices[indices]
            x0 = max(0, int(np.floor(tri[:,0].min())), int(np.ceil(clip[0])))
            x1 = min(width, int(np.ceil(tri[:,0].max())), int(np.ceil(clip[2])))
            y0 = max(0, int(np.floor(tri[:,1].min())), int(np.ceil(clip[1])))
            y1 = min(height, int(np.ceil(tri[:,1].max())), int(np.ceil(clip[3])))
            if x1 <= x0 or y1 <= y0: continue
            a,b,c = tri[:,:2]
            den = (b[1]-c[1])*(a[0]-c[0])+(c[0]-b[0])*(a[1]-c[1])
            if abs(den) < 1e-8: continue
            yy,xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)+.5
            wa = ((b[1]-c[1])*(xx-c[0])+(c[0]-b[0])*(yy-c[1]))/den
            wb = ((c[1]-a[1])*(xx-c[0])+(a[0]-c[0])*(yy-c[1]))/den
            wc = 1-wa-wb
            mask = (wa >= 0) & (wb >= 0) & (wc > 0)
            if not mask.any(): continue
            weights = np.stack([wa[mask],wb[mask],wc[mask]],axis=1)
            uv = weights @ tri[:,2:4]
            color = (weights @ tri[:,4:])/255
            tx = np.clip(uv[:,0]*tw-.5,0,tw-1)
            ty = np.clip(uv[:,1]*th-.5,0,th-1)
            ix,iy = tx.astype(int),ty.astype(int)
            jx,jy = np.minimum(ix+1,tw-1),np.minimum(iy+1,th-1)
            fx,fy = (tx-ix)[:,None],(ty-iy)[:,None]
            sampled = (texture[iy,ix]*(1-fx)+texture[iy,jx]*fx)*(1-fy)+(texture[jy,ix]*(1-fx)+texture[jy,jx]*fx)*fy
            rgba = color*sampled
            tile = target[y0:y1,x0:x1]
            tile[mask] = rgba+tile[mask]*(1-rgba[:,3:4])
    Image.fromarray(np.uint8(np.clip(target[:,:,:3]*255,0,255))).save(path.with_suffix('.png'))
    print(path.with_suffix('.png'))


if __name__ == '__main__':
    import sys
    for path in (map(Path,sys.argv[1:]) if len(sys.argv)>1 else Path('/tmp/market-monitor-qa').glob('*.json')):
        render(path)
