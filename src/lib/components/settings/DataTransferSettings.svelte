<script lang="ts">
    interface Props {
        onimport?: () => void;
        onexport?: () => void;
    }

    let { onimport, onexport }: Props = $props();
</script>

<section class="field" aria-label="数据管理">
    <span class="field-label">收藏导入导出</span>
    <div class="transfer-row">
        <button type="button" class="ghost-btn" onclick={() => onimport?.()}>
            导入
        </button>
        <button type="button" class="ghost-btn" onclick={() => onexport?.()}>
            导出
        </button>
    </div>
    <small>文件格式为 JSON，导入采用增量模式</small>
</section>
