import {FstJsReader} from './index.js';

/*
export declare function readFst(file: string): string
export declare function getVarValueAtTime(file: string, varName: string, time: number): number
export declare function getNextTimeChange(file: string, varName: string, startTime: number): number
export declare function getTimescale(file: string): string
export declare function getTimezero(file: string): number
export declare function getMetadata(file: string): object
export declare function getVariableInfo(file: string, varName: string): object*/


const reader = new  FstJsReader("/home/strau/repos/gym/ma/computer/build/fib.fst")

console.log(reader.read())
let i = 200;
while (i < 500) {
    console.log(`I: ${i}, ${reader.getVarValueAtTime( "TOP.cpu.rax_op [1:0]", i)}`)
    console.log(`I: ${i}, ${reader.getVarEnumValueAtTime( "TOP.cpu.rax_op [1:0]", "control::reg_op_e", i)}`)
    i++;
}
console.log("future");
console.log(reader.getNextTimeChange("TOP.cpu.control_unit.alu_op [3:0]", 105))
console.log(reader.getTimescale())
console.log(reader.getTimezero())
console.log(reader.getMetadata())
console.log(reader.getVariableInfo("TOP.cpu.alu.op [3:0]"))
