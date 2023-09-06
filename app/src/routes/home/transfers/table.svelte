<script lang="ts">
    import {ChevronDown} from "svelte-ionicons";
    import {invoke} from "@tauri-apps/api"

    import {settings} from "../../../settings";
    import Resizable from "../../../components/resizable/resizable.svelte";

    import { Columns, displayColumnsSelector } from "./columns"
</script>

<table>
    <thead>
    <tr on:contextmenu|preventDefault={(e) => {
          displayColumnsSelector({ x: e.clientX, y: e.clientY}, $settings.transfers.columns);
    }}>
        {#each $settings.transfers.columns as [column, width]}
            <th style="width: {width}px;" on:mouseup={() => {
                if($settings.transfers.sort[0] !== column) {
                    $settings.transfers.sort = [column, "asc"];
                } else {
                    $settings.transfers.sort[1] = $settings.transfers.sort[1] === "asc" ? "desc" : "asc";
                }
            }}>
                <Resizable limits={[30, 1500]} bind:width={width} height="20px" resize_cords={["right"]}>
                    {#if $settings.transfers.sort[0] === column}
                        <div class="sort">
                            <ChevronDown
                                    style="transform: rotate({$settings.transfers.sort[1] === 'asc' ? '0deg' : '180deg'}); opacity: 75%"
                                    size="12px" color="#000"/>
                        </div>
                    {/if}
                    {Columns[column]}
                </Resizable>
            </th>
        {/each}
    </tr>
    </thead>
</table>

<style>
    .sort {
        position: absolute;
        top: -8px;
        left: 50%;
        transform: translateX(-50%);
    }

    table {
        border-collapse: collapse;
        user-select: none;
    }

    th {
        border-left: 1px solid #bbb;
        border-right: 1px solid #bbb;

        font-size: 13px;
        font-weight: normal;
        color: #000;
    }

    th:first-child {
        border-left: none;
    }


    th:hover {
        background-color: rgba(30, 144, 255, 0.2);
    }
</style>