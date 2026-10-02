import { registerSW } from "virtual:pwa-register";
import { getCollection } from "astro:content";

export const pwa = [registerSW, getCollection];
