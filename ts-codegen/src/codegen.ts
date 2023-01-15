import codegen, { ContractFile } from "@cosmwasm/ts-codegen";
import { glob } from "glob";

glob("../@(contracts|packages)/**/schema", (err, matches) => {
  if (err) console.log(err);

  let contracts: ContractFile[] = [];

  matches.forEach((x) => {
    let split = x.split("/");
    console.log(split[split.length - 2]);
    contracts.push({
      name: split[split.length - 2],
      dir: x,
    });
  });

  codegen({
    contracts,
    outPath: "./output",
  }).then(() => {
    console.log("✨ all done!");
  });
});
