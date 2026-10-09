import base from "/home/lainterus/Projects/High Grade/ui/playwright.config.ts";
export default {...base,testDir:"/home/lainterus/Projects/High Grade/ui/tests",outputDir:"/home/lainterus/Projects/High Grade/target/highgrade/ui-playwright",use:{...base.use,baseURL:"http://127.0.0.1:6007"}};
