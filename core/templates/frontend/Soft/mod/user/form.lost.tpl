<div class="clear-line"></div>
<form action="{site_url}/index.php?dn=user" method="post">
<div class="form-area">
    <fieldset>
        <legend>{rest_pass_hint}</legend>
        <input class="width" name="lostmail" size="30" type="text" maxlength="255" />
    </fieldset>
    <div class="pad ac">
        <input name="re" value="lost" type="hidden" />
        <input name="to" value="send" type="hidden" />
        <button type="submit" class="sub">{send_pass}</button>
    </div>
    <div class="clear"></div>
</div>
</form>
