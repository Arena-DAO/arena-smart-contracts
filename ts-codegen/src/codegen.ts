import codegen, { ContractFile } from "@cosmwasm/ts-codegen";
import { glob } from "glob";
import { join } from "path";

glob("../@(contracts)/**/schema").then((matches) => {
  let contracts: ContractFile[] = [];

  for (const x of matches) {
    let split = x.split("\\");
    let name = split[split.length - 2];
    let dir = join(__dirname, "..", x).replaceAll("\\", "/");

    console.log(dir);
    console.log(name);
    contracts.push({
      name: name,
      dir: dir,
    });
  }

  codegen({
    contracts,
    outPath: "./output",
    options: {
      bundle: {
        bundleFile: "index.ts",
      },
      types: {
        enabled: true,
      },
      client: {
        enabled: true,
        execExtendsQuery: true,
      },
      reactQuery: {
        enabled: true,
        optionalClient: true,
        version: "v4",
        mutations: true,
        queryKeys: true,
        queryFactory: true,
      },
    },
  }).then(() => {
    console.log("✨ all done!");
  });
});
