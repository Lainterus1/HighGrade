import {defineConfig} from '@playwright/test';
export default defineConfig({testDir:'.',testMatch:'performance.ts',outputDir:'../../target/highgrade/tmp/performance-results',timeout:180000,workers:1,reporter:'list',use:{headless:true,viewport:{width:1440,height:900},launchOptions:process.env.HIGHGRADE_CHROME?{executablePath:process.env.HIGHGRADE_CHROME}:undefined}});
