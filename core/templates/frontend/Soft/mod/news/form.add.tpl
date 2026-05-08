<script src="{site_url}/javascript/jquery.form.js"></script>
<script>
$(document).ready(function() {
    $('#text').textarearesizer();
    $('#refresh').click(function() {
        var t = new Date().getTime();
        $('#divcaptcha').html('<img src="{site_url}/image.php?to=captcha&t=' + t + '" alt="" />');
    });
});
</script>
<form action="{site_url}/index.php?dn=news" method="post">
<div class="comment">
    <fieldset class="standart">
    <legend>{title}:</legend>
        <strong>|</strong>
        <input class="width" name="title" type="text" />
    </fieldset>
    <fieldset class="standart">
    <legend>{in_cat}:</legend>
        <strong>|</strong>
        <select class="width" name="catid">
        {sel}
        </select>
    </fieldset>
    <fieldset class="standart">
    <legend>{text}:</legend>
        <textarea class="width" cols="60" rows="10" id="text" name="text"></textarea>
    </fieldset>
    <fieldset class="standart">
    <legend>{image}</legend>
        <input class="width" name="image_thumb" type="text" />
    </fieldset>
    <fieldset class="standart">
    <legend>{thumb}</legend>
        <input class="width" name="image" type="text" />
    </fieldset>
    <!--if:captcha:yes-->
    <fieldset class="standart">
    <legend>Captcha</legend>
    <table class="wpc_100">
    <tbody>
        <tr>
            <td class="wpc_100">
                <strong>|</strong>
                <input class="width" id="captcha" name="captcha" type="text" maxlength="5" />
            </td>
            <td class="ac va pad">
                <div id="divcaptcha"><img src="{site_url}/image.php?to=captcha" alt="" /></div>
            </td>
            <td class="ac va pad">
                <button type="button" id="refresh" class="sub">{all_refresh}</button>
            </td>
        </tr> 
    </tbody>
    </table>
    </fieldset>
    <!--if-->
    <!--if:control:yes-->
    <fieldset class="standart">
    <legend>{control_word}</legend>
        <p>{control}</p>
        <strong>|</strong>
        <input class="width" id="respon" name="respon" size="30" type="text" />
        <input name="cid" type="hidden" value="{cid}" />
    </fieldset>
    <!--if-->
    <div class="pad ac">
        <input name="re" value="add" type="hidden" />
        <input name="to" value="save" type="hidden" />
        <button type="submit" class="sub">{all_add}</button>
    </div>
    <div class="clear"></div>
</div>
</form>
