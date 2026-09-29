import React from 'react';
import {setNonce} from 'get-nonce';
import {createRoot} from 'react-dom/client';
import {NativeWorkspace} from '@/features/specs/NativeWorkspace';
import '@/styles/theme.css';
const styleNonce=document.querySelector<HTMLMetaElement>('meta[name="highgrade-style-nonce"]')?.content;
if(styleNonce)setNonce(styleNonce);
createRoot(document.getElementById('root')!).render(<React.StrictMode><NativeWorkspace/></React.StrictMode>);
