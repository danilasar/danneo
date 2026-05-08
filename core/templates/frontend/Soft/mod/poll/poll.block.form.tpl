<div class="poll-block-title">{title}</div>
<form action="{site_url}/index.php?dn=poll" method="post">
<div class="poll-block">
    {percent}
</div>
<div class="pad ac">
    <input name="id" value="{id}" type="hidden" />
    <input name="re" value="add" type="hidden" />
    <button type="submit" class="sub">{button}</button>
</div>
</form>
<!--buffer:percent:0-->
<div class="poll-radio">{radio} {val_name}</div>
<!--buffer-->
